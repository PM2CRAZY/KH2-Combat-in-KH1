# KH2 knockback on enemies

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from your message in KH2's combo row s](#^m1)

---


> [!tip] Assistant
> I started this thread from your message in [KH2's combo row selection](../kh2s-combo-row-selection/kh2s-combo-row-selection.md): "it seems like guard breaks knock back is wrong (it was sending the enemy really far) please make a separate thread for investigating and implementing kh2's knockback on enemies". Research only so far, no code. Traces and logs for this go in `testers/kh2-knockback-on-enemies/`.
>
> **How KH2 does it.** Sora and every enemy go through one reaction routine (KH2 Steam `0x1403D2FE0`). It sorts the attack's reaction into a group (`0x1403CE980`):
> - **Push only** (reactions 1, 0xC).
> - **Flinch plus a push** (3, 6, 8, 10, 0xD, 0x10, 0x12). In the air it's the air flinch: a 50 hop, drifting at the push distance × 0.048.
> - **Launch, a "blow"** (4, 7, 9, 0xB, 0xF, 0x11). It plays the blown-away animation (from the front or from behind) and its landing. The attack's first knockback number is the upward speed and the second the sideways speed (`0x1403CEA00`); only when both are 0 does KH2 use its defaults, 24 up and 40 sideways. In flight, each 30 Hz tick the sideways speed is ×0.8 (never below a tenth of the start), the rise ×0.8, then gravity 20/49, the fall capped at 16.
> - **Magnet's pull.**
> - **The push distance** (`0x1403CEAB0`): the attack's first knockback number, or 200 if it has none, starting at a tenth of it per frame and shrinking ×0.9 each frame.
>
> The numbers come straight from the attack, not the enemy: I found no weight or per-enemy factor in that routine (the objentry weight only splits how two bodies push each other apart).
>
> **Guard Break in KH2.** Its blow is attack 876: a launch, 20 up, **0 sideways**. So in KH2 it pops the enemy straight up where it stands, about 117 high and back down in about 56 frames, with no slide.
>
> **What this branch does now.**
> - A KH2 launch counts as a "throw" only if its sideways speed is at least 10, or its up speed at least 30, or both are 0. A throw goes through KH1's launch (`0x1402B8530`), and we then write KH2's height, rise time and sideways speed over KH1's.
> - Any other launch is treated as a "holding blow", a rule made up for Aerial Finish (its blows are 5–12 up and −1 / −10 sideways): KH1's plain flinch in place, then for 24 frames the enemy is pulled toward a point near Sora (at most 90 from him, 30 above), up to 30 a frame.
> - Guard Break's 20 up / 0 sideways lands in that holding group. Guard Break dashes Sora about 250 units (your traces show it), and the hold drags the enemy along with him. The traces don't record enemy positions, so the drag is my reading of the code, not something I measured, but nothing else in that path moves the enemy.
> - Other differences:
>   - flinch hits use KH1's own push (80 × an enemy stat), not KH2's push distance;
>   - in flight after a launch KH1 keeps its own curve and its own per-enemy slowdown;
>   - Explosion's last blast has a hand-tuned override (from an earlier request, not KH2's numbers);
>   - about a dozen of Sora's KH2 attacks with small numbers are caught by the same "holding" rule.
>
> **The native plan.** KH1's own enemy reactions stay the only system; we feed them KH2's numbers.
> 1. **Launches:** every KH2 launch goes through KH1's launch with KH2's starting speeds; the "holding" rule goes.
> 2. **Flight:** a KH2-launched enemy flies by KH2's blow rules, through KH1's enemy air step (I still need to confirm its Steam address), and lands into KH1's own landing and get-up.
> 3. **Flinches:** KH2's push distance and its 0.1 / 0.9 curve in KH1's enemy push (`0x1402B7E60`), and KH2's air flinch hop and drift in KH1's air flinch (`0x1402B8B90`).
> 4. **Enemies KH1 can't launch** (no launch animation; the big and fixed ones): KH1's own fallback stays, as now.
>
> One system per version. A launched enemy costs one small step per frame, only while it's in the air.
>
> Questions:
> 1. KH1 scales its pushes and launches by a per-enemy stat (enemy stats `+0xC4`); KH2 doesn't. (a) KH2's numbers for every enemy, or (b) KH2's numbers × KH1's per-enemy factor?
> 2. KH1 Sora is about 0.89× KH2 Sora's size. (a) KH2's distances as they are, or (b) scaled by 0.89?
> 3. Aerial Finish's "hold" and Explosion's farther blast came from earlier requests and aren't KH2's numbers. (a) drop both for KH2's real values, (b) keep both, or (c) drop the hold but keep Explosion's?
>
> If that sounds right, say go ahead with your answers and I'll start with the launches. A trace of a Guard Break and an Aerial Finish on one enemy, then on a group, would be ideal.

^m1

