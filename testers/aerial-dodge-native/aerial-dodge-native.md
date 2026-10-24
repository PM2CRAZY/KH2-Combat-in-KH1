# Aerial Dodge native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) because you asked for one thread per item. This one is the Aerial Dodge. I've only done research here so far, no code.
>
> **What we have now.** A fresh jump in the air with Aerial Dodge equipped is a borrow: KH1 gives Sora to KH2's controller, which does the dodge and its fall, and two frames into the fall hands him back to KH1's fall state. That hand-over is where the old glitches came from (one frame of KH1's jump or glide pose at the start, the landing request at the hand-back).
>
> **What KH2 does** (Steam KH2 decomp, `0x1403CB9F0`). The jump press in any air action reads Aerial Dodge's growth level. It needs the "dodge left" bit, which the air actions only set when you came from the ground. It clears that bit, asks for motion 202, and launches with the level's height and speed along the stick (straight up with the stick neutral). The apex switches to the fall (motion 4) and the landing is 5. Gravity is 20/49 as usual. The numbers come from your `03system.bin`: LV1 145 high and 18 a frame, LV2 155 / 24, LV3 200 / 30, MAX 300 / 36. Motion 202 is a short loop with a guard on his body (ATKP 406, it stops hits with no reaction) all the way through, it's "air free" from frame 4, and it plays sound 999019 on frame 1. There are no invulnerable frames.
>
> **How KH1 would carry it.** KH1 has no double jump, but its air state (2) is already where the air combo's hops and Aerial Recovery run natively. KH1's launch (`0x1402AF960`) sets the arc fields `+0x3B0..+0x3BC` and zeroes the jump's motion base `+0x3D0`. KH1's apex swap (`0x1402AF630`) only fires for its own jump requests. The air state's pre-step (`0x1402AFC20`) skips grabs during a rise and faces the stick. Its jump press (`0x1402AFD60`) is the glide, or flight in Neverland.
>
> **The plan, one system per version:**
> 1. The dodge as a KH1 request of our own with KH2's motion 202, the same way Aerial Recovery was done in t475. KH1's air state stays in charge: our request is asked, he's turned to the stick, and KH2's hop numbers go into KH1's air fields. It's still once per air time, and the launch speed is still kept for KH2's glide. No borrow and no hand-back.
> 2. The fall where KH2 puts it, at the apex, asked by us since KH1's own apex swap won't fire for our request.
> 3. The 406 guard on the native slot. Hits already stop for our slots, but the parry against enemy attacks (KH1 guard objects) needs a path that doesn't go through KH2's controller.
> 4. The early release, if KH2's dodge turns out to be holdable. I still need to confirm that in KH2's held-jump code.
>
> **What I couldn't pin down:** whether walking off a ledge gives you the dodge in the Steam build (our notes say it doesn't, but they came from the Epic exe). Also which KH1 slot to build 202 on: a jump slot fits better than the air-swing slot Aerial Recovery uses, but I'll know once I hear it.
>
> Full notes: `docs/kh1-aerial-dodge-native.md`.
>
> Questions:
> 1. Where should the fall pose take over? (a) at KH2's apex, like KH2, or (b) as soon as 202's air-free frame 4 has passed and he starts dropping.
> 2. Out of KH2's glide (Glide at KH2 is still a borrow), should the dodge (a) stay KH2's controller's until the glide goes native, or (b) end the glide borrow and go native?
> 3. In Neverland the jump in the air is KH1's flight. Should it be (a) the dodge first and flight on the next press, like now, or (b) flight first, so no dodge in Neverland?

^m1

1. unsure you pick what would make it feel 1:1 while making native and having less strain on performance compared to what we have now
2. (b) full native, glide will get converted
3. (a)

go ahead
