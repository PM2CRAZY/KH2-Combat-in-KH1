# KH2 magic and Reflect native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: start working on "- KH2 magic and Reflect: a borrow, plus ou](#^m1)
- [2. Assistant: Moved here from "What is still missing", so that thread stay](#^m2)
- [3. Assistant: t477 and t478. Started on native magic.](#^m3)

---


> [!note] Mr Bugs
> start working on "- KH2 magic and Reflect: a borrow, plus our own copy of KH2's spell scripts and our own spell hit checks. Native would be KH1's cast state with spell objects KH1 hits itself. Large; I first have to work out how KH1 makes its own spells." if it hasn't already been done. make it a separate thread from this one so I can keep this post clean 
>
> go ahead
>
> *update, also make sure that we aren't pulling data from the main branch, still want to get everything integrated with test branch first*

^m1


> [!tip] Assistant
> Moved here from "What is still missing", so that thread stays clean. Started: first I'm working out how KH1 makes, moves and hits its own spells, then moving KH2's magic and Reflect onto that one system at a time (KH1's cast state with KH2's cast animations, spells as KH1's own attack objects hit by KH1's own collision, Cure's heal, Reflect). Nothing from the main branch is used, only the testing branch, the decomps, the exes and the KH2 files. I'll post the versions here as they're done, with questions where there's a choice.

^m2


> [!tip] Assistant
> **t477 and t478.** Started on native magic.
>
> - **t477:** Fire, Blizzard, Thunder and Cure at Magic / Aerial Magic KH2 are now cast by KH1's own command step and cast state, with no handoff to KH2's controller. KH2's cast animations play as KH1 animation slots, KH1's own spell script is skipped and KH2's runs on KH1's frames, and the spell comes out on KH2's cast frame.
> - **t478:** a cast that lands on the combo's finisher step uses KH2's finisher cast (Fire 57, Blizzard 60, Thunder 63) with its finisher voice.
>
> **Check:** the KH1 decomp and exe for KH1's cast (the command step, the spell start, its script calls and the collision pass) and the KH2 files for the cast animations and MAGC. Not checked in game. How KH1 casts is written up in `docs/kh1-magic-native.md`.
>
> Not native yet: the hits, Reflect, Cure's heal, magic into magic, the combo going on after a cast, and KH2's stop/hover during a cast. Those need your call:
> 1. After a cast in a combo: (a) the magic window lets anything follow and the cast counts as a step, (b) only magic may follow in it but the combo continues after the cast, or (c) a cast ends the combo?
> 2. Movement while casting: (a) KH2's stop/hover when physics is KH2, (b) KH2's whenever Aerial Magic is KH2, or (c) KH1's?
> 3. KH1's attack shapes are only spheres, and some KH2 spells hit with ovals (Firaga's ring is 240 wide × 180 tall): (a) one big sphere, (b) several spheres acting as one hit (closest to KH2), or (c) a thin sphere swept along the long side?
> 4. Reflect as KH1's own guard: (a) bounce enemy shots back like KH1 does, (b) just stop them, or (c) keep blocking every hit like now?
> 5. Cure's heal: (a) KH2's formula with max MP standing in for Magic, (b) KH1's own Cure amount, or (c) keep the current share of max HP?
> 6. MP: KH2 costs Fire 12, Blizzard 15, Thunder 18, Cure all, Reflect 10. (a) keep KH1's 1 MP per spell, (b) scale KH2's costs onto KH1's MP, or (c) a KH2 MP pool?
>
> About the main branch: nothing reads data from the main folder. The current magic code itself (the spell runtime and part of the KH2 spell-script crate) was carried over from main when the testing branch was made; the native version replaces it step by step.

^m3

