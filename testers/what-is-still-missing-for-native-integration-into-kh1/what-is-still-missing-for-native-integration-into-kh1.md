# What is still missing for native integration into KH1

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: what else currently are we missing for native integration in](#^m1)
- [2. Assistant: Here's what isn't native yet, grouped. Anything already bein](#^m2)
- [3. Assistant: Done: every item that isn't native yet has its own thread no](#^m3)
- [4. Assistant: Got it, thanks for spelling it out. I've added your definiti](#^m4)
- [5. Assistant: Here's the list from my last message, updated with this roun](#^m5)

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


for everything that isn't native yet start a new thread for each and start the research into each, use the kh1 and kh2 decomps (steam c code)

go ahead

> [!tip] Assistant
> Done: every item that isn't native yet has its own thread now, each with a first round of research from the KH1 and KH2 Steam decomps (and the exes and KH2 files where the decomps didn't show it). Research only, no code yet. Each thread ends with a few questions; answer them there and give the go-ahead when you're happy with the plan.
>
> - [Aerial Dodge native](../aerial-dodge-native/aerial-dodge-native.md): it can go native the way Aerial Recovery did in t475, a request of ours on KH1's air state with KH2's hop numbers.
> - [Dodge Roll, Guard and Counterguard native](../dodge-roll-guard-and-counterguard-native/dodge-roll-guard-and-counterguard-native.md): KH1 has no guard state; its guard is an attack row (a priority-100 guard), so KH2's guard can go on that row.
> - [Quick Run native](../quick-run-native/quick-run-native.md): KH1's state table has free ids, so Quick Run can be a KH1 state of its own.
> - [KH2 Glide in KH1's glide state](../kh2-glide-in-kh1s-glide-state/kh2-glide-in-kh1s-glide-state.md): what wasn't 1:1 in t284 to t320, and KH1's glide state mapped piece by piece.
> - [Drive Forms native](../drive-forms-native/drive-forms-native.md): the gauge first (KH2's gauge, form gauge and orbs mapped), then one form at a time on base Sora's native pieces.
> - [KH2 attack data and damage on KH1's swings](../kh2-attack-data-and-damage-on-kh1s-swings/kh2-attack-data-and-damage-on-kh1s-swings.md): both damage formulas found; KH1 attack data records built from KH2's ATKP.
> - [Sora's hit reactions native](../soras-hit-reactions-native/soras-hit-reactions-native.md): KH1's hit pass mapped, plus a gap in how borrows skip KH1's 30-frame no-damage timer.
> - [Baking the rest of the base animations](../baking-the-rest-of-the-base-animations/baking-the-rest-of-the-base-animations.md): the KH1 requests and slots for idle, jump, fall, landing, glide, items and the Gravity / Stop / Aero casts.
> - [KH2 sounds through KH1's sound system](../kh2-sounds-through-kh1s-sound-system/kh2-sounds-through-kh1s-sound-system.md): KH1 and KH2 use the same sound engine on PC, so KH1 should be able to load KH2's banks.
> - [KH2 player model through KH1's model system](../kh2-player-model-through-kh1s-model-system/kh2-player-model-through-kh1s-model-system.md): a second KH1 model instance carrying KH2's skeleton.
> - [KH2 hit effects on enemies](../kh2-hit-effects-on-enemies/kh2-hit-effects-on-enemies.md): KH2's hit-spark pick maps straight onto KH1's own lookup.
> - [KH2 MP and MP Charge](../kh2-mp-and-mp-charge/kh2-mp-and-mp-charge.md): KH2's rules on KH1's own MP pool; ties into your (6b) in the magic thread.
> - [KH2 Limits](../kh2-limits/kh2-limits.md): KH2's limit system and KH1's limit states, which already use the rows path.
> - [KH2 summons](../kh2-summons/kh2-summons.md): which KH2 summon rules can run on KH1's own summons.
> - [Reaction commands](../reaction-commands/reaction-commands.md): KH1's Triangle-box context commands could host KH2's RCs natively.
>
> Not given threads of their own, because they already have one: KH2's ability moves and their hits (the combo rows thread), magic and Reflect (its own thread), and camera, Frame Advance, effects, sleeves and the Config menu. Epic isn't a native item, so it's still waiting on question 6 from my last message.
>
> The notes for each are in `docs/` (named in each thread). Everything in them is tagged Checked (decomp / exe / files / code) or Assumption.
>
> Two things that touch more than one thread:
> - The summons' KH2 cost (3 Drive bars) needs the Drive gauge from the Drive Forms thread.
> - The MP thread and your (6b) in the magic thread are one shared cost scale. I'll build it once.

^m3


currently I've noticed some agents getting around native implementation or not understanding what I mean by native, what I mean by native is giving the kh1's engine more abilities so that it can do everything kh2 can so that kh2 is 1:1 inside of kh1. what currently is technically "native" and runs along the kh1 version, what currently is using the deprecated player. and can you please update the instructions and memory so that its understood what I mean by 100% native. (to my understanding our effects is what I want, giving kh1's effect engine the ability to do kh2's effects properly)


> [!tip] Assistant
> Got it, thanks for spelling it out. I've added your definition to the instructions: `docs/design-philosophy-instructions.md` ("100% NATIVE") and `docs/project-instructions.md` ("What native means"), which every helper brief now points to. In short: native means giving KH1's engine new abilities (new states, rows, requests, motion slots, effect programs, sound banks, camera modes, functions patched into KH1's own ones) so KH1 runs KH2's thing itself. The effects work is the model. A KH2 system of ours running next to KH1's and copying its results in doesn't count, even if it never borrows Sora. (I can't write the project's memory from here; the instruction files are what every session reads, and I've asked the owner to paste the updated project instructions.)
>
> Here's where everything stands by that rule.
>
> **Native (KH1's own system doing KH2's thing)**
> - Effects through KH1's effect engine, with KH2's missing programs added to it; KH2's hit sparks through KH1's own hit lookup.
> - Sounds and battle quotes through KH1's sound engine (the old player is gone since t565).
> - Animations baked as KH1 motion slots: the combos, walk and run, the KH2 casts, the ability swings, the glide's motions.
> - Quick Run as a KH1 state, Aerial Dodge on KH1's air state, the dodge roll on KH1's roll, the guard on KH1's guard row, KH2 Glide in KH1's glide state, the body guards as KH1 guard objects.
> - Most ability moves on KH1 attack rows; KH2's power inside KH1's damage function; KH2's flinches, pushes and recoils on KH1's reaction requests; the casts in KH1's cast state.
> - The KH2 camera as a mode in KH1's camera table.
>
> **Running along KH1 (our own KH2 logic beside KH1's system)**
> 1. Row picks: the KH2 ability rows, the Square moves, Counterguard and Retaliating Slash are still checked by a separate KH2 object, not inside KH1's picker. The [combo rows thread](../kh2s-combo-row-selection/kh2s-combo-row-selection.md) now has the plan to fix this (one walk in KH1's picker).
> 2. Body hits (Vicinity Break, Explosion, Magnet Burst): our own sweep of KH2's shapes, then KH1's contact. KH1's own collision pass doesn't see them.
> 3. The KH2 spells after the cast: their scripts (Reflect's barrier and counter, the shots) run in our own interpreter, and their hits go through our own records, not KH1 spell objects ([magic thread](../kh2-magic-and-reflect-native/kh2-magic-and-reflect-native.md)).
> 4. Idle, jump, fall, landing and item use are still drawn over KH1's pose every frame ([baking thread](../baking-the-rest-of-the-base-animations/baking-the-rest-of-the-base-animations.md)).
> 5. Our own hood, coat and shorts rules ([sleeves thread](../missing-sleeves-animation/missing-sleeves-animation.md)), and KH2's MP costs on KH1's MP with no MP Charge yet ([MP thread](../kh2-mp-and-mp-charge/kh2-mp-and-mp-charge.md)).
>
> **Still on the deprecated KH2 controller or renderer**
> 6. Drive Forms, all of them ([Drive Forms thread](../drive-forms-native/drive-forms-native.md)).
> 7. The fallbacks: when a native path can't run (a slot not built, a state KH1 doesn't cast in, flight, or Epic), KH2's controller still borrows Sora. Some ability moves still hit this; your t544 log had Vicinity Break borrowed. I can't list exactly which ones without a trace.
> 8. Hits taken during a borrow or a form are decided and played by KH2's controller.
> 9. The KH2 player model: our own renderer, switched off ([model thread](../kh2-player-model-through-kh1s-model-system/kh2-player-model-through-kh1s-model-system.md)).
>
> Limits, summons and reaction commands aren't in at all yet, so they're on neither list.
>
> Questions:
> 1. Which do you want converted first? (a) the row picks (1), since it's planned already and every combo goes through it; (b) the body hits (2) and the fallback borrows (7), so base Sora never touches KH2's controller; (c) the spells (3), as KH1 spell objects; (d) the remaining animations (4); (e) Drive Forms (6, with 8), the biggest one; or (f) another order, just list it.
> 2. When a native version needs something KH1 has no system for at all (MP Charge, for example): (a) build it into KH1's engine, the way Quick Run got a new state, or (b) ask you each time first?

^m4


give me an update 
> [!tip] Assistant
> Here's the list from my last message, updated with this round (t621 to t631):
>
> **Moved to native**
> - 1, row picks: done for the combo. KH2's pick is made inside KH1's picker (t622, t623), and every KH2 combo move, plain swings included, is now its own KH1 attack row that the picker returns (t631).
> - 7, fallbacks: in the combo, KH2's controller no longer takes Sora. When no KH1 row can play a move, KH1's own picked row plays (t630). Other fallbacks (flight, Epic, Drive Forms) are unchanged.
> - 4, animations: item use, the fall, jump and High Jump are KH1 slots now (t625 to t627). Idle and the landing are still drawn over, waiting on one answer in the baking thread.
> - The magic combo: Finishing Plus after a finisher cast (t624).
>
> **Still running along KH1:** 2 (body hits for Vicinity Break, Explosion and Magnet Burst), 3 (the KH2 spells' scripts and hits), the rest of 4 (idle, landing), 5 (hood / coat / shorts rules, MP Charge).
>
> **Still on the deprecated controller or renderer:** 6 Drive Forms, 8 hits taken during a form, 9 the KH2 player model.
>
> My two questions from last time still stand: which to convert next ((b) body hits, (c) spells, (d) idle and landing, (e) Drive Forms, or another order), and whether a missing KH1 system like MP Charge should just be built into KH1's engine (a) or asked about each time (b).

^m5

(b), (c), make sure its the responses are updated in the threads

go ahead