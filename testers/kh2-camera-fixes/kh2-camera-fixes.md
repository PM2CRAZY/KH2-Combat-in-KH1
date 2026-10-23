# KH2 Camera fixes

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: I noticed when you run backwards the camera starts turning s](#^m1)
- [2. Assistant: t373](#^m2)
- [3. Mr Bugs: 1. it seems right but check the ghidra kh2 steam c decomp to](#^m3)
- [4. Assistant: t379 to t384 (one change each; the build at the top of the b](#^m4)
- [5. Mr Bugs: kh2 "Found, not changed (KH2's lock-on turn speed is a const](#^m5)

---


  

> [!note] Tester Mr Bugs
> I noticed when you run backwards the camera starts turning sora in a circle and rotates this is not indented, can you please look through the kh2 decomp to verify and fix this change then if you need the kh2 ghidra c decomp steam version to find any other missing features do it. I've logged a trace for you to see whats going on in game
> 
> go ahead

^m1


> [!tip] Assistant
> t373
>
> Your trace showed it exactly. With the stick held straight down, our camera read it as pushing forward. So it kept swinging round behind Sora at 1° a frame, and since Sora runs relative to the camera, he turned with it and ran in a circle. The cause was the stick's up/down being passed to the KH2 camera the wrong way round.
>
> **What KH2 does.** I checked KH2's camera code in the Steam KH2 decomp (the follow update at `0x140165110`). KH2 doesn't swing the camera while you're running almost straight at it, at nearly full tilt. If you run diagonally backwards, or pull back only part way, KH2 does still swing slowly, so you'll get a gentle curve there. That's KH2's own behaviour, and ours now does the same.
>
> **Check:** the turn per frame in your trace matches KH2's swing formula exactly, and the condition was read in the KH2 decomp. Not checked in game yet. Notes are in `docs/kh1-kh2-camera.md` (9a) and `docs/camera-model.md`.
>
> Two questions:
> 1. Please try running straight back and diagonally back. Does the diagonal curve feel like what you remember from KH2?
> 2. **Stick dead zone.** KH2 ignores about the first half of the stick's tilt (60 of 127.5 on each axis). Ours ignores 15%, because gentle tilts used to do nothing. At full tilt they're the same, but at part tilt the swing differs. (a) switch to KH2's exact dead zone for 1:1, or (b) keep ours?
>
> Other things I found in KH2's camera, not changed yet: the right stick may turn the camera at full speed even when tilted diagonally (I need a closer read before I'd change it), there's a reset branch whose purpose I don't know yet, and I haven't read KH2's zoom code. Tell me if you want any of those looked at.

^m2


> [!note] Tester Mr Bugs
> 1. it seems right but check the ghidra kh2 steam c decomp to make sure
> 2. it should be 1:1 
> 
> yes, look at everything related to the camera that your unsure about, use ghidra kh2 steam c decomp for it.
> 
> go ahead

^m3


> [!tip] Assistant
> t379 to t384 (one change each; the build at the top of the branch has them all)
>
> I went back through the KH2 decomp like you asked, and I got one thing wrong last time. KH2 looks at which way the stick points, not how far it's tilted. So running straight back never swings the camera, even at part tilt, as long as you're within about 26° of straight back. Further round than that (diagonally back), it still swings slowly. Ours now does exactly that (t379).
>
> Also done, all from KH2's code (Steam KH2 decomp):
> - **t380:** the camera uses KH2's exact stick dead zone (60 of 127.5 on each axis, so about the inner half), read the way KH2's pad code reads it. KH1's own sticks are built the same way, so the keyboard and mouse still work as before.
> - **t381:** KH2 movement now reads the stick the same way, so very gentle tilts won't walk Sora any more, like KH2. This changes how gentle tilts feel, so tell me if anything's off.
> - **t382:** the right stick turns and zooms by its whole tilt, so a full diagonal turns and zooms at full speed together, like KH2.
> - **t383:** a reset (R3) puts the eye straight on its goal and only the turn eases; while you're moving it's an instant cut, with no drift after.
> - **t384:** faster forms swing and follow faster, from the form's own run speed (before, it was fixed at Sora's).
>
> **Check:** decomp for all of them; not checked in game. Assumptions: which pad bits KH2's turn and zoom test are in the exe's data tables, which the decomp doesn't show, so that they're the right stick's x and y is an assumption, and that the run speed KH2's camera reads is the form's. Notes are in `docs/camera-model.md`, `docs/kh1-kh2-camera.md` and a new `docs/kh2-pad.md`.
>
> Found, not changed (KH2's lock-on turn speed is a constant I can't read here; ours is 3°, and KH2 scales turning by the mouse's motion, which ours doesn't):
> 1. **KH2 has its own first-person view:** Sora fades out over 30 frames, you look from his head 350 units ahead, the right stick tilts the view up and down, and he fades back in when you leave it. Ours uses KH1's first person on Select. (a) switch to KH2's first person, or (b) keep KH1's?
> 2. **Walk / run split:** which tilt KH2 counts as a run isn't confirmed yet (we use half tilt). Want me to look it up in the decomp next?
> 3. **An unknown button** can stop KH2's auto-swing and allow zoom in places it's normally blocked; which button it is sits in KH2's remap tables. Want me to track it down?

^m4


> [!note] Tester Mr Bugs
> kh2 "Found, not changed (KH2's lock-on turn speed is a constant I can't read here; ours is 3°, and KH2 scales turning by the mouse's motion, which ours doesn't):" Make sure your not mixing up the mouse camera and the controller camera, those are seperate cameras (at least from a user's experince, unsure how it is in code)
> 1. (a)
> 2. yes
> 3. unsure what an "unknown button" is but yes go ahead and do it
>    
>    go ahead

^m5


