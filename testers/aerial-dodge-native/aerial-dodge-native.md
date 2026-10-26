# Aerial Dodge native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Thanks! For question 1 I went with (a), the fall at KH2's ap](#^m2)
- [3. Assistant: Thanks, and thanks for the traces! They showed exactly what ](#^m3)
- [4. Assistant: Thanks! Done, (b) it is.](#^m4)

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


1. unsure about the levels, check the decomp
2. currently sora in kh1 mode cancels his air dodge, that doesn't fit with how kh2 does air dodge in the base game. I recorded a trace will be in the same folder as this thread
3. none its fixed now
4. seems good
5. working here
6. I don't hear any
7. (a) go and investigate those moves and ensure that they have the proper frame data and timings in its own thread

go ahead


> [!tip] Assistant
> Thanks, and thanks for the traces! They showed exactly what was going on.
>
> - **t545: the dodge isn't cut short at Physics KH1 anymore.** In both traces, every dodge got cut at its 6th frame into KH1's little release hop (5 frames, about 6 units), and that happened even while you were still holding jump. That's KH1's own variable jump. KH1 tracks two flags on Sora: "the cut is still open" and "jump still held". The dodge kept the flags from the jump before it. In your traces you'd let go of that first jump for a moment, so KH1 thought the button was already released and cut the dodge as soon as its window opened. Now the dodge sets both flags fresh, the same way KH1's own takeoff does. KH1's variable jump then works on the dodge like it does on a jump: hold it and you get the full rise; let go between 6 frames in and 5 before the apex and the rise is cut, then the fall starts right away. That's the same window KH2 uses for its dodge, just with KH1's hop because Physics is on KH1. Physics KH2 isn't affected. Checked: KH1's button handler `0x1402A7B30` and takeoff `0x1402AFDA0` in the decomp, KH2's `0x1403CB680` window, and your two traces.
> - **The levels are right, so no change.** I checked KH2's decomp: `0x1403CB9F0` reads your Aerial Dodge level, and `0x1403E92D0` picks that level's record and reads the height at +4 and the speed at +8. I also read those values straight out of your `03system.bin`: LV1 145 high / 18 a frame, LV2 155 / 24, LV3 200 / 30, MAX 300 / 36. That matches what the mod uses. Your traces show 145, so LV1. One thing to know: at Physics KH1 the same height takes a little less time to reach, since KH1's gravity is a bit stronger.
> - **Walking off a ledge:** you get the dodge there, same as KH2. KH2 has no separate fall action. Walking off puts Sora in its jump action, and entering that from the ground gives you the dodge. The mod already did this. One caveat: "walking off uses the jump action" is my reading, since KH2 has no fall action at all. I didn't find the exact call.
> - **The 160-frame slot is fine.** The dodge's animation only plays up to the fall swap, about 33 frames at MAX, so 160 is plenty.
> - Also settled: the dodge out of the glide and the hover uses KH2's own glide tables (checked by class name in the exe). Before, that was a guess.
>
> Your 7 (a): the body guards on Flash Step, Aerial Recovery, Explosion, Vicinity Break and Magnet Burst have their own thread now, "[Body guards on KH2 moves](../body-guards-on-kh2-moves/body-guards-on-kh2-moves.md)". The frame data and timings for those moves get checked there.
>
> What I'd like tested (Physics KH1, a trace each if you can):
> 1. Jump, let go, then dodge while holding jump. It should rise all the way.
> 2. Dodge and tap jump. It should be cut short, with the fall starting right as you let go.
> 3. Walk off a ledge and dodge.
>
> One question: with Physics KH1, the dodge now has KH1's early release (tap = short, hold = full). In t495 I'd said there was no release at KH1. Which do you want?
> (a) Keep the release, with KH1's hop (what t545 does, and closest to KH2's dodge).
> (b) No release at Physics KH1: the dodge always rises fully.

^m3


(b)

go ahead

> [!tip] Assistant
> Thanks! Done, (b) it is.
>
> - **t597: the dodge always rises fully at Physics KH1.** KH1 has a flag on Sora that means "the jump can still be cut short", and its variable jump only cuts a rise while that flag is on. The dodge now turns that flag off as it starts, so KH1 never cuts it: hold jump or let go, you get the full height, and the fall still starts near the top like KH2's. Physics KH2 isn't touched; there the dodge keeps KH2's own release, as in KH2. Checked: KH1's button handler `0x1402A7B30` in the Steam decomp. That flag is the only thing the cut looks at, and no other KH1 code reads it on Sora. Not checked yet: in game.
>
> What I'd like tested (Physics KH1, a trace each if you can):
> 1. Dodge and tap jump right away. It should still rise all the way, with the fall starting near the top.
> 2. Jump, let go, then dodge. The old cut 6 frames into the dodge shouldn't show up anywhere.
> 3. A normal KH1 jump right after a dodge, tapped and then held. KH1's own short and full jump should still work as before.
>
> No open questions on my side.

^m4

I noticed that air dodge right now you seem to not be able to cancel it into a glide as fast as you can in kh2, can please check and see against the c steam decomp if its letting you? if you see the reason why then implement it.

go ahead