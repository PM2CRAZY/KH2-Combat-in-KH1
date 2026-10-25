# KH2 spell hit sounds

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from KH2 hit effects on enemies(../kh2](#^m1)

---


> [!tip] Assistant
> I started this thread from [KH2 hit effects on enemies](../kh2-hit-effects-on-enemies/kh2-hit-effects-on-enemies.md), your answer 2 (b) there: give KH2's spells their own ATKP hit sound through KH1's sound path.
>
> What it is: each KH2 attack row (ATKP) names the sound its hit plays. Right now a KH2 spell's hit (KH2-style Fire / Blizzard / Thunder, Wisdom's shots, Reflect's pop) plays KH1's hit sound for the attack it lands with. The plan is to have KH1's own contact code play the KH2 row's sound instead, the same way t521 hands it the spell's hit spark, using the sound banks that now load into KH1's engine (t565 to t567 in the sounds thread).
>
> Nothing is coded yet. Before I start I'll check, in the KH2 decomp, where KH2 reads the hit sound from the row and which bank it plays from, and in the KH1 decomp, where KH1's contact plays its hit sound.
>
> Question: should this also cover KH2 swings' hit sounds (the Keyblade's own KH2 hit sound, next to the t560 spark), or only spells? (a) spells only, as asked; (b) spells and KH2 swings.
>
> Say "go ahead" (with the answer) when you want it built.

^m1


