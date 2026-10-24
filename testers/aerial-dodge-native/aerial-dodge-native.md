# Aerial Dodge native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Thanks! For question 1 I went with (a), the fall at KH2's ap](#^m2)

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

> [!tip] Assistant
> Thanks! For question 1 I went with (a), the fall at KH2's apex. It's what KH2 does, and it's the cheaper one too: one check a frame on Sora, no new hook. All five steps are in. With the borrow gone, KH2's controller doesn't run for the dodge anymore.
>
> - **t492, the dodge on KH1's air state.** The fresh jump in the air is now a KH1 request of ours playing KH2's motion 202. There's no hand-over and no hand-back. He turns to the stick and gets KH2's hop for your Aerial Dodge level (145/18, 155/24, 200/30, 300/36). It's once per air time, and the launch speed is still kept for KH2's glide. KH1's air step steers it with the stick like a jump, the way KH2 does. The slot loops 202 the way KH2 loops it, and only KH2's sound (999019) plays on it. I dropped the swish from the KH1 template slot. If the request isn't built, or he's in a Drive Form, it falls back to the old borrow. Checked: KH2's handler in the decomp (`0x1403CB9F0` / `0x1403CBB00`).
> - **t493, the fall at the apex.** He goes into KH1's fall 5 frames before the apex with an 8-frame blend, the same as KH2 (`0x1403CB680` → `0x1403CB610`, decomp). KH1's own apex swap only fires for KH1's own jump, so this one is ours. Landing is still KH1's.
> - **t494, the guard.** 202's guard on his body (ATKP 406) now also becomes a KH1 guard object on the native slot, so enemy attack objects get parried with no reaction. Before this it only stopped hits his body took. Checked: 202's window and 406 in your files.
> - **t495, the early release.** KH2's dodge is holdable after all; I confirmed it in the KH2 decomp (`0x1403CBD30` turns the dodge's flag into the held-jump bit, and `0x1403D6FC0` / `0x1403CB680` cut it). Let go between 6 frames in and 5 before the apex and the rise cuts to KH2's release hop, with the fall starting right then like KH2. That's with Physics at KH2; at KH1 there's no release, same as before.
> - **t496, out of KH2's glide.** Your 2 (b). In KH2's glide (still a borrow for now), a jump press with the dodge left ends the glide borrow and starts the native dodge.
>
> Neverland is your 3 (a): the dodge takes the first press and KH1's flight takes the next, same as before.
>
> What I couldn't check: whether 160 frames of slot covers every rise. I think it does, since MAX tops out around 77. I also couldn't check which of KH2's 17 air-action tables is the glide's. And still open from before: whether walking off a ledge gives you the dodge in the Steam build.
>
> What I'd like tested (a trace with each helps):
> 1. A dodge with the stick held, then one with it neutral. Is the height right for your level? It should go straight up when neutral.
> 2. Tap jump for the dodge, then hold it. The tap should be clearly shorter, and the fall pose should start right as you let go.
> 3. Do you see any KH1 jump or glide pose on the first frame, or a landing pose at the start of the fall? Those were the old borrow glitches.
> 4. Dodge into an enemy's attack (a Shadow's swipe, a thrown ball). It should be stopped with no reaction.
> 5. Glide with KH2's glide, then press jump. You should get the dodge right out of the glide.
> 6. Is there any KH1 air-swing swish on the dodge? There shouldn't be.
>
> One question: other moves have the same kind of guard on his body (Flash Step's 406, Aerial Recovery's 124, Explosion, Vicinity Break, Magnet Burst). On their native slots it still only stops hits his body takes; it doesn't parry. Should those get the parry too, like KH2? (a) yes, all of them; (b) leave them for their own threads.

^m2


