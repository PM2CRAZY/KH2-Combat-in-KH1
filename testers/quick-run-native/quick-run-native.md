# Quick Run native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) at your request. This one is Quick Run.
>
> **What we have now.** Quick Run is a borrow: KH2's controller takes Sora for it (Square held through KH1's roll, or Square while moving without KH1's Dodge Roll) and moves him through KH1's mover. Its numbers are KH2's, except one thing we changed on purpose back in stage 104: LV1 gets 6 invulnerable frames, and KH2's LV1 has none.
>
> **What KH2 does.** In KH2's exe it's the `ACTION_AIRSLIDE` action (vtable `0x1405CDB40`). The start (`0x1403A9830`) takes the fmab record for Sora's Quick Run level and sets the speed, the brake, the hold timer, the stop brake, motion 203 and the invulnerable time. Each update, the timer only counts down while Square is held, and it drops to 0 the moment you let go. At 0 the brake switches to the stop brake and motion 204 plays. Each update the speed is multiplied by the brake. From your 03system.bin:
> - LV1: time 14, speed 30, brake 0.94, invulnerable 0
> - LV2: 18, 34, 0.95, 8
> - LV3: 40, 38, 0.96, 12
> - MAX: 40, 38, 0.98, 16 (stop brake 0.93 at every level)
>
> **How KH1 does the matching thing.** KH1 has no Quick Run. Its dodge roll isn't a state of its own either: it's KH1's ground state (0) with an action flag, motion 0xDC and a 12-frame timer. The motion's events turn on the untouchable flag, and state 0's ground step moves him. KH1's movement states live in a table at `0x142DB3840`: 59 slots of 0x40 bytes each (flags, enter, exit, pre-step, step, transition), filled by `0x1402B1B80`. These ids are free (never registered, never compared by number): 6, 0x23, 0x24, 0x2A-0x31, 0x33-0x3A. Things that read the state number:
> - the physics step
> - KH1's camera (it treats only 0 / 8 / 0x18 as "standing")
> - the command menu (the state's flags decide which commands it offers)
> - a few switches in the hit and command code
>
> Event scripts don't read it; they only set it.
>
> **The plan (one system per version):**
> 1. Quick Run as its own KH1 state in a free slot. Its enter, step and transition would run KH2's start, hold, brakes and end with the fmab numbers above. The speed would be kept in KH1's momentum field (`+0x3C0`), so leaving a ledge or landing carries it on. The invulnerability would go through KH1's untouchable flag, and the motions through KH1's motion requests.
> 2. The start points (Square held through the roll, Square while moving) start that state instead of borrowing KH2's controller.
> 3. The end: KH2's landing, or a fall with Aerial Dodge allowed when he goes off a ledge, on KH1's own transition.
>
> The other route is KH1's roll pattern: state 0 plus an action, with the brake and timer added into state 0's step. That avoids a new state number, but state 0 snaps him to the floor and ends the action when he leaves the ground, which might not be what KH2 does.
>
> **What I couldn't find yet:**
> - whether KH2's air slide keeps gravity or sticks to the floor, and exactly how its end motion hands over to the landing
> - KH2's update rate on PC, which decides the brake's value per 60 Hz frame
> - what KH2 starts for levels above LV1 (a call with level + 0x6C; probably an effect)
>
> Notes are in `docs/kh1-quick-run-native.md`.
>
> Two things I'll settle from the decomp before building rather than ask you: which commands KH1's menu offers during a Quick Run (whatever can cut KH2's), and whether it holds him off the floor at a ledge. Both will follow KH2.
>
> Question:
> 1. LV1's invulnerability: (a) KH2's real 0 frames (1:1), or (b) keep the 6-frame minimum you asked for in stage 104?

^m1


