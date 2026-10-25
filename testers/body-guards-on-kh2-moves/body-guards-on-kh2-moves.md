# Body guards on KH2 moves

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from Aerial Dodge native's question 7,](#^m1)

---


> [!tip] Assistant
> I started this thread from Aerial Dodge native's question 7, where you picked (a): give the other moves' body guards the parry, and check their frame data and timings.
>
> t546: every KH2 guard window on Sora's body now becomes a KH1 guard object, the same way the Aerial Dodge's did in t494. KH2's controller already does this for every guard window, with no exceptions, so now KH1's states do too. On their native slots that means:
> - Aerial Recovery: ATKP 124, frames 0..16
> - Explosion: ATKP 124, frames 32..64
> - Magnet Burst: ATKP 406, frames 20..70
> - Aerial Dodge: ATKP 406, the whole motion (no change)
>
> An enemy attack that meets one gets stopped there. A thrown shot gets turned back, and the enemy recoils by KH1's rule. Sora gets no reaction and the move keeps going, which is KH2's "refact self 2". Hits that reach his body in those frames were already stopped, and still are.
>
> How I checked: I read every one of these moves' windows from your P_EX100.mset and W_EX010.mset, and the ATKP rows from 00battle.bin. I also checked KH2's range test in the Steam decomp (0x14040E740). A window is live from its start frame up to, but not including, its end frame, and that's how our slots read it. Everything the code uses matches the files:
> - Invulnerable frames: Aerial Recovery 0..16, Explosion 32..64. Magnet Burst, Flash Step and Vicinity Break have none in KH2.
> - Guard frames: as in the list above.
>
> One assumption: 124 is KH2's "special guard" (it also stops pierce-armor attacks) and 406 is a plain guard. KH1's enemy attacks don't have KH2's attack kinds, so here both act the same.
>
> What I found while checking:
> - Flash Step and Vicinity Break aren't on KH1's states at all yet. Your t544 logs never show them baked, and Vicinity Break shows "KH2 borrows him for it". That's because they also have guard windows on the Keyblade:
>   - Flash Step: 162 at 17..22, then 161 at 22..30. The 161 window is the real guard, so a hit there starts KH2's guard repel.
>   - Vicinity Break: 162 at 14..32.
>
>   The native slot builder doesn't accept moves with Keyblade guards. Through the borrow, KH2's controller plays them with all their guards and the right frames, but it isn't native.
> - Explosion and Magnet Burst both have one more KH2 range (id 44) on exactly the same frames as their guards. I haven't found what reads it in the KH2 decomp yet.
>
> What I couldn't check: anything in game.
>
> What I'd like tested (a trace with each helps):
> 1. Explosion into an enemy's swing during its spin. The swing should stop with a clank, and Sora shouldn't react.
> 2. Magnet Burst the same way, during its pull.
> 3. Aerial Recovery right into an enemy attack, in its first 16 frames.
> 4. A thrown shot (Wakka's ball, a Soldier-type projectile) into Explosion or Magnet Burst. It should be stopped or turned back.
>
> Questions:
> 1. Flash Step and Vicinity Break: (a) put them on KH1's states natively in this thread, including their Keyblade guards and Flash Step's repel window, or (b) leave them for the combo thread, since that one decides which KH2 ability moves KH1's combo plays natively?
> 2. Range id 44 on Explosion and Magnet Burst: (a) I keep digging in the decomp for what it does, or (b) it can wait unless you notice something off in those two moves.

^m1


