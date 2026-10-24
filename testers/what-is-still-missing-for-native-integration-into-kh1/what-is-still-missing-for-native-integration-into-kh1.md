# What is still missing for native integration into KH1

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: what else currently are we missing for native integration in](#^m1)
- [2. Assistant: Here's what isn't native yet, grouped. Anything already bein](#^m2)

---


  

> [!note] Mr Bugs

> what else currently are we missing for native integration into kh1?
>

^m1


> [!tip] Assistant
> Here's what isn't native yet, grouped. Anything already being done in one of your open threads is marked so it doesn't get done twice. Sizes are rough: small is a version or two, medium a few, large needs research first.
>
> **Moves still on KH2's controller (the borrows)**
> - KH2's ability moves (Explosion, Magnet Burst, Vicinity Break, Guard Break, Counterguard, Finishing Leap, Upper Slash, Horizontal Slash, Retaliating Slash, Aerial Recovery): already in the combo rows thread.
> - Aerial Dodge: could run on KH1's air state with KH2's hop numbers, like the air combo now does. Small.
> - KH2's dodge roll, guard and Counterguard: would go on KH1's own roll and guard states with KH2's animations and invulnerability frames. Medium; I still need to find KH1's guard state.
> - Quick Run: KH1 has no state for it, so it would need a new KH1 state or KH1's roll state reused. Medium.
> - KH2 Glide: you made it a borrow in t321 because the shared version wasn't 1:1. Native would mean putting KH2's whole glide logic into KH1's glide state, not just its numbers. Medium.
> - KH2 magic and Reflect: a borrow, plus our own copy of KH2's spell scripts and our own spell hit checks. Native would be KH1's cast state with spell objects KH1 hits itself. Large; I first have to work out how KH1 makes its own spells.
> - Drive Forms: still entirely KH2's controller (movement, combos, hits, reactions, Wisdom's shots, Final's glide). Also missing: a Drive gauge (forms cost MP now), form levels (growth abilities are always MAX), Anti, Master and Limit forms, and Drive orbs. The biggest item on the list.
>
> **Hits and reactions**
> - Borrowed and form moves use our own hit checks against KH1's enemies. These go native along with the moves above.
> - The native swings use KH1's attack data (damage, reaction, element, hit effect), not KH2's, and damage is KH1's damage times KH2's power everywhere. Medium.
> - When Sora gets hit in KH1's states he plays KH1's reactions; in borrows and forms KH2's controller plays KH2's. Native would be KH1's reaction pass playing KH2's reaction animations and numbers. Medium to large.
> - KH2 hit effects on the enemy being hit aren't built (probably fits the effects thread).
>
> **Animations**
> - Baked into KH1's own slots already: the combos, walk, run and a few ability swings. Still drawn over KH1 every frame: idle, jump, high jump, fall, landing, glide, item use, and the Gravity / Stop / Aero casts. Small each. The skeleton and coat are in the sleeves thread.
>
> **Our own drawing and sound**
> - The KH2 player model is drawn by our own renderer instead of KH1's model system. Large; KH1's model format and the two skeletons don't match.
> - KH2 sounds and KH2 battle quotes play through Windows directly, so KH1's volume, pause and Frame Advance don't affect them. Native would load KH2's sound banks into KH1's sound system. Medium.
>
> **Already in their own threads**: camera, Frame Advance, effects, combo rows, sleeves and skeleton, Config menu.
>
> **Not in at all**: KH2's MP and MP Charge, Limits, KH2 summons, reaction commands.
>
> **Epic**: the Config "KH1 in KH2" menu, Frame Advance, the R2 / mouse camera and the ledge hop are Steam only. Since the old effect player is gone, a lot of KH2's effects don't play on Epic at all. I can't fix that without the Epic exe.
>
> Questions:
> 1. Which do you want to start on? (a) Aerial Dodge, (b) dodge roll, guard and Counterguard, (c) Quick Run, (d) KH2 Glide in KH1's glide state, (e) magic and Reflect, (f) Drive Forms plus the gauge, (g) KH2's attack data on the native swings, (h) Sora's hit reactions, (i) baking the rest of the base animations, (j) KH2 sounds through KH1, (k) the KH2 model through KH1.
> 2. Should each of those be its own thread, or (a) group them: moves (a to d), magic (e), forms (f), hits (g and h), animation (i), sound and model (j and k)?
> 3. Damage: (a) keep KH1's damage times KH2's power, or (b) try KH2's own damage formula? KH1 doesn't have all of KH2's stats, so it would need some stand-ins.
> 4. KH2 Glide: (a) keep it as KH2's own move, like you chose in t321, or (b) put KH2's full glide logic into KH1's glide state?
> 5. Quick Run: (a) add a new KH1 state for it, or (b) reuse KH1's roll state?
> 6. Epic: (a) Steam only is fine, or (b) you can send the Epic exe so I can map the missing addresses?

^m2

