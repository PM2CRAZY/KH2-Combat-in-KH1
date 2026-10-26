# Quick Run native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: t501 to t505. Quick Run is its own KH1 movement state now (s](#^m2)
- [3. Assistant: Thanks for the trace, it showed it clearly.](#^m3)
- [4. Assistant: Thanks! (a) it is, so nothing changes: with Dodge Roll on KH](#^m4)
- [5. Assistant: Yes, it's in, and it's from this branch, not main: t505 adde](#^m5)
- [6. Assistant: Thanks, that narrowed it down. At MAX there's nothing to see](#^m6)
- [7. Assistant: t665: Debug Menu > Growth Abilities.](#^m7)

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


do your first purposed route. 

1. make behave exactly 1:1 with kh2 so if its (a) then do it
   
   also its getting hard to read which doc gets updated, can you add an extra thread that tells me whats updated and whats needed from me? 
   
   go ahead

> [!tip] Assistant
> **t501 to t505.** Quick Run is its own KH1 movement state now (state 0x2A, a free slot in KH1's state table), so KH2's controller no longer takes over. Everything runs on KH2's numbers from your 03system.bin.
>
> - **t501, the state:** KH2's start speed and its hold timer. The timer only counts down while Square is held, and letting go stops the run at once with KH2's stop brake and its stop motion (204). The brake is applied every second frame, matching KH2's physics tick. KH1's own ground movement carries him, so floors and slopes are KH1's. The speed sits in KH1's momentum field, and when the stop motion ends you get KH1's landing.
> - **t502, the start points:** Square held through KH1's roll, Square while moving, and KH2's own Square order with Dodge Roll on KH2 (guard at rest, then roll, then Quick Run) all start the state now. The borrow is only left as a fallback for Drive Forms.
> - **t503, off a ledge:** he drops into a fall from rest with the full slide speed, and he still has his Aerial Dodge, as in KH2.
> - **t504, invulnerability 1:1:** your (a). LV1 has 0 frames like KH2; LV2 / LV3 / MAX get 8 / 12 / 16. The old 6-frame minimum is gone.
> - **t505, the start effect:** at LV2 and up, KH2 plays a small effect when the run starts (elements 110 to 112). That's in now.
>
> From the decomp: nothing can cut a KH2 Quick Run except letting go of Square. Its two motions have no "free" window, and KH2's jump and command checks need one, so jump and the command menu do nothing mid-run.
>
> **Check:** the Steam KH2 and KH1 decomps, and motions 203 and 204 in your files. **Not checked:** how KH2 hands the stop motion over to its landing, and which animation KH2 shows when you run off a ledge.
>
> About the extra thread you asked for: it's [Status: what's updated and what's needed from you](../status-whats-updated-and-whats-needed-from-you/status-whats-updated-and-whats-needed-from-you.md). I'll update it every round.
>
> Please test:
> - a tap, then a full hold at each level;
> - running off a ledge, then an Aerial Dodge;
> - getting hit during a run at LV1 and at LV2+;
> - Square held through KH1's roll;
> - slopes and stairs.
>
> Questions:
> 1. Running off a ledge mid-run, KH2's fall doesn't ask for an animation of its own. Should he (a) switch to the fall animation at once (what's in now), or (b) keep the Quick Run animation until it ends?
> 2. The LV2+ start effect: (a) on by default (now), or (b) only with all KH2 effects on?

^m2


quick run seems to work on kh2 dodge roll with kh1 on the other hand it:
waits until dodge roll finishes to go into quick run
doesn't get the same distance

I recorded a trace and sent you the logs its in the same folder as this thread is (quick-run-native)
make sure it behaves the same as kh2 dodge roll

go ahead


> [!tip] Assistant
> Thanks for the trace, it showed it clearly.
>
> **t547, Quick Run out of KH1's roll at KH2's frame.** In your trace KH1's roll started at frame 9384 and the Quick Run only at 9414, 30 frames later, which is the end of KH1's roll animation. With the KH2 roll it starts 10 frames in. KH2's rule is that its roll turns into Quick Run at a marker in the roll animation, frame 10, while Square is held. Our check reached frame 10 with KH1's roll too, but it then waited for KH1 to let Sora act, and KH1 only does that once its roll is over. Now KH1's roll is cut at frame 10, the same frame and counter as the KH2 roll.
>
> The Quick Run part was already the same both ways: in your log both start at speed 30 and stop after 13 frames at 19.5.
>
> Check: your trace frame by frame, the KH2-roll timings in your log, and KH2's roll animation (motion 209) read from your files. Not checked: in game.
>
> **Distance.** Most of the gap was the 20 extra frames of KH1's roll. What's left is the roll itself before the cut. KH2's roll covers about 221 units in its first 10 frames, KH1's about 160 (17.8 a frame, and its first frame doesn't move). That's KH1's own roll speed, so I left it alone with the row on KH1.
>
> Please test, with Dodge Roll KH1 and then KH2:
> - roll and hold Square: the Quick Run should start at the same point both ways;
> - roll and let go before the Quick Run starts;
> - a Quick Run at each level you have.
>
> Question:
> 1. The roll part before the Quick Run, with the row on KH1: (a) keep KH1's own roll speed (about 60 units shorter than KH2's by the cut, KH1 stays exactly KH1), (b) when Square is held, move him by KH2's roll travel for those 10 frames so the whole thing matches KH2's distance, or (c) something else?

^m3


(a) this is fine 

go ahead

> [!tip] Assistant
> Thanks! (a) it is, so nothing changes: with Dodge Roll on KH1 the roll before the Quick Run keeps KH1's own roll speed (about 60 units shorter than KH2's by the cut), and KH1 stays exactly KH1. The Quick Run itself already starts at KH2's frame from either roll (t547), and its speed and length are the same both ways.
>
> No new version for this one. If anything about the roll into Quick Run still feels off in game, a trace of it with Dodge Roll on KH1 and then KH2 would show it.

^m4


did you implement the quick run double effect? I currently don't see it in my build while the person your using to accesses files can, is that from the main branch?
> [!tip] Assistant
> Yes, it's in, and it's from this branch, not main: t505 added it, and main has no Quick Run effect at all. The person whose files I read is on t657 of `testing`, the same line as your build.
>
> I think the difference is the Quick Run level, not the build. KH2 only plays that start effect from LV2 up: its Quick Run start (`0x1403A9830` in the Steam KH2 decomp) starts element "level + 0x6C" only when the level is above 1. That's P_EX100's element 110 at LV2, 111 at LV3 and 112 at MAX, and nothing at LV1. We do the same.
> - In your log in this folder (t544), every Quick Run is **LV1**: "KH2's Quick Run LV1 … invulnerable 0". So no effect plays, the same as in KH2.
> - In the other logs I can see, the Quick Runs are **LV3 and MAX**, so the effect plays.
>
> Quick Run's level follows Sora's level in this mod: LV1 from 12, LV2 from 20, LV3 from 32, MAX from 45. Drive Forms add one level on top.
>
> Checked: the Steam KH2 decomp (the level test and the element number), our code (`quickrun.rs`, `kh2fx`'s default list has motion 203), main's effect list (no 203), and both sets of logs. Not checked: how the effect looks in game, since I can't see the screen. If "double effect" means something other than this start effect (two copies of it, or an afterimage on Sora), tell me and I'll look again.
>
> Please test:
> - A Quick Run on a save where Sora is level 20 or higher (LV2+), with `kh2_effects` on its default. The start effect should show as the run begins.
>
> Question:
> 1. Is that the effect you saw on the other build: (a) yes, the burst at the start of a LV2+ Quick Run; or (b) something else (describe it, or send a clip or trace)?

^m5


I didn't see it on quick run max, but james saw it on level 3, unsure about level two can you make it a setting in the debug where I can adjust the growth abilities 
> [!tip] Assistant
> Thanks, that narrowed it down. At MAX there's nothing to see, and that's KH2's behaviour, not a bug of ours:
> - KH2's Quick Run start (`0x1403A9830`) asks for element "level + 0x6C": 110 at LV2, 111 at LV3, 112 at MAX. The function that starts it (`0x1402C6B30`, through `0x1403B6670`) looks through Sora's element list for that id, and if none matches it plays nothing. That's from the Steam KH2 decomp.
> - Base Sora's effect file (`P_EX100.a.us`, its `p_ex` list) has 110 and 111, but no 112. I parsed the file to check, and James's in-game log agrees: "p_ex100 element 112 doesn't play" on his MAX runs.
> - So in KH2 too, base Sora's Quick Run shows the start effect at LV2 and LV3 only. James saw it at LV3; you ran MAX, so you didn't.
>
> **A debug setting for growth abilities:** yes, that fits the debug menu. Here's the plan, then a couple of questions before I build it.
> - A new "Growth Abilities" page in the debug menu, with one row each for High Jump, Quick Run, Dodge Roll, Aerial Dodge and Glide.
> - Each row reads: Normal (what your level and story give you now) | Off | LV1 | LV2 | LV3 | MAX.
> - The override replaces the level the mod works out from your level (`growth.rs`), so the moves themselves, their numbers (KH2's fmab records), the Quick Run start effect, the invulnerable frames and the menu name all follow it. Nothing is written into your save.
> - Like the rest of the debug menu, it only loads with `debug_menu = on` in the ini (t663).
>
> Questions:
> 1. Should the overrides (a) last for this session only (back to Normal on the next start), or (b) be saved in the ini like the other debug settings?
> 2. In Drive Forms, the form's own growth ability is one level above base Sora's. With an override set, should the form (a) still add its +1 on top of the override, or (b) use the override as it is?
> 3. With a row set to a level, should Sora (a) have the ability even if he hasn't learned it or doesn't have it equipped (simplest for testing), or (b) only when he has it equipped, with the level replaced?

^m6


1. the overrides can always be adjusted in the debug, keep it
2. don't worry about drive forms
3. (a)
this is purely for testing, the debug menu should be able to break any rule when we add a feature to figure out situations.

go ahead
> [!tip] Assistant
> **t665: Debug Menu > Growth Abilities.**
> - There's a row each for High Jump, Quick Run, Dodge Roll, Aerial Dodge and Glide. Left and right go through Normal / Off / LV1 / LV2 / LV3 / MAX, and confirm puts a row back to Normal. Each row also shows the level Sora has right now.
> - A level you pick works even if Sora hasn't learned or equipped the ability (your (a)). Off turns it off even if he has it.
> - Your choices are saved in the ini (`debug_high_jump`, `debug_quick_run`, `debug_dodge_roll`, `debug_aerial_dodge`, `debug_glide`), so they stay across restarts. They only exist with `debug_menu = on`; with it off, nothing is overridden.
> - The override replaces the level at the one place the mod hands Sora his growth abilities, so everything reads it: KH2's numbers for the level (Quick Run's speed, brake and timer, its LV2+ start effect, the invulnerable frames, the dodge roll, Aerial Dodge, the glide, the High Jump height) and the menu names. Nothing goes into your save.
> - Drive Forms are left alone, as you said. A form uses its own growth ability, and your overrides come back when you revert.
> - One limit: with a style row on KH1 (KH1's own jump, roll or glide), KH1's own move follows KH1's ability list, not this page. Stats > Abilities still controls those.
>
> Checked: every reader of these levels in our code. Not checked: in game (how the page looks with its rows, and the feel). The "now" column for Glide may show 0 when the glide style is KH1's, even though the glide uses your forced level.
>
> Please test:
> - Quick Run at LV1, LV2, LV3 and MAX: the start effect should show at LV2 and LV3 only. Base Sora has no MAX effect, as in KH2.
> - Off on something you've learned, and a level on something you haven't learned yet.
> - Restart, and check your choices are still set.

^m7


