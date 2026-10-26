# What is still missing for native integration into KH1

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: what else currently are we missing for native integration in](#^m1)
- [2. Assistant: Here's what isn't native yet, grouped. Anything already bein](#^m2)
- [3. Assistant: Done: every item that isn't native yet has its own thread no](#^m3)
- [4. Assistant: Got it, thanks for spelling it out. I've added your definiti](#^m4)
- [5. Assistant: Here's the list from my last message, updated with this roun](#^m5)
- [6. Assistant: Here's this round for (b) and the first part of (c). Each on](#^m6)
- [7. Assistant: Here's this round:](#^m7)
- [8. Assistant: Here's this round.](#^m8)
- [9. Assistant: Thanks! Two versions this round:](#^m9)
- [10. Assistant: Thanks! (a) it is, so nothing changes: on starts where the c](#^m10)
- [11. Assistant: Here's the list brought up to date, from t632 to t675.](#^m11)
- [12. Assistant: Thanks! This is a big list, so I'm doing it one system per v](#^m12)
- [13. Assistant: Thanks! Here's how I took your answers, then this round's tw](#^m13)
- [14. Assistant: Thanks! I worked through Valor, then moved Wisdom over too. ](#^m14)
- [15. Assistant: Thanks! All nine are in. 1 (a) and 5 (a) need no change: the](#^m15)

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
> [!tip] Assistant
> Here's this round for (b) and the first part of (c). Each one also has its own reply in its topic thread.
>
> - **t638, body hits.** Vicinity Break, Explosion and Magnet Burst's hits on Sora's body are now KH1 attack objects that KH1's own collision pass collides, with KH2's shapes and hit numbers. Counterguard's and Finishing Leap's body hits too. Our own sweep for them is gone. More in the [combo rows thread](../kh2s-combo-row-selection/kh2s-combo-row-selection.md).
> - **t639, button fallbacks.** When KH1's states of ours can't take a Square (dodge roll / guard / Quick Run) or an air jump (Aerial Dodge), KH2's controller no longer borrows Sora. The press is KH1's, the way t630 did it for the combo.
> - **t640, spell hits.** Every KH2 spell's attacks are now KH1 attack objects made with KH1's own constructor (the one KH1's spell scripts use), and KH1's collision pass collides them. More in the [magic thread](../kh2-magic-and-reflect-native/kh2-magic-and-reflect-native.md).
>
> Checked: KH1's collision pass, the attack constructor and the row maker in the Steam KH1 decomp; the hooked bytes in the exe; the shapes, windows and re-hit intervals in KH2's files. Not checked: in game.
>
> Still borrowing base Sora:
> 1. An Attack pressed during one of KH1's own moves (a KH1 spell, an item, KH1's roll at Base Combo KH2): KH2's controller still plays that next swing. My plan is to give the Attack to KH1's own Attack step (KH1 already plays KH2's picks since t622 / t631), with the spell counted as a combo step the way KH2 casts already are. It touches the spell-cut rules from the magic thread, so I'd do it as its own version next unless you say otherwise.
> 2. A KH2 spell (Magic KH2, Reflect, Gravity / Stop / Aero on KH2's animation) chosen in a state where KH1 casts nothing: KH1 only casts standing, walking, jumping, falling or flying. Also a spell picked during KH1's own jump, glide or roll.
> 3. Attack while flying in Neverland still uses KH2's flight combo.
> 4. The Drive and everything in a form (the Drive Forms thread).
>
> Please test:
> - Vicinity Break, Explosion and Magnet Burst on groups of enemies: damage, Explosion's re-hits, the Magnet pull, the finisher launches. Log: "KH2 body hits: motion …", "KH2 body hit: ATKP …".
> - Fire / Blizzard / Thunder / Cure / Reflect's counter hitting enemies. Log: "KH2 spell hits: ATKP … on KH1 attack object …".
> - Square in deep water, and Square / air jump with Dodge Roll, Guard, Quick Run or Aerial Dodge not equipped. Log: "no KH1 state of ours could take it here".
>
> Questions:
> 1. A spell chosen in a state KH1 can't cast in (2 above): (a) refuse it with KH1's "can't" sound, as KH1 would; (b) teach KH1's cast step to cast in those states too (tell me which: swimming, ladders, the slider…); or (c) something else?
> 2. Attack while flying: (a) KH1's own flying Attack, with KH2's air combo picks on KH1's air rows (KH1 keeps its flight physics); or (b) the same, but with KH2's flight-combo rules (no sinking, homing at the target) built into KH1's flight state?
> 3. Still open from last time: when a native version needs something KH1 has no system for at all (MP Charge, for example), (a) build it into KH1's engine, or (b) ask you each time?
> 4. Should I go ahead with 1 (an Attack during KH1's own moves going to KH1's Attack step) as the next version?

^m6


1. (b) if you have magic set to kh2 those magics should be 1:1, the reverse is true regardless of the animation.
2. (a) if there's a more elegant implementation to let the animations work as flight air attacks e.g if the game has a different animation for sora when attacking in the flight state then we'll just give kh2 more animations, if not then find how it works and give kh2 the ability.
3. (a) should always be built into the engine; I want to state that the goal for this project is to have as little as running in our code every frame to reduce the amount of bugs, performance issues, etc. So what that entails is having the game build the assets it needs from kh2 and putting it in its mod folder, it won't get destroyed when the game is shut off like it does now but instead is built from a players extracted data from kh1 and kh2. 
4. not sure if this is updated let me know
   
   go ahead

> [!tip] Assistant
> Here's this round:
>
> - **4, Attack during KH1's own moves:** that one is already done on current testing. For base Sora in KH1's own states, an Attack during a KH1 spell, an item or KH1's roll goes to KH1's Attack step, which plays KH2's picks (t622 / t631); KH2's controller isn't involved. It can still happen in three cases: a KH1 command started while KH2's controller already had him (the borrows that are left: a KH2 spell in a state KH1 can't cast in, and Drive Forms), Aerial Base Combo KH2 if the air slots failed to build, and Epic. My list last time was out of date there, and the same goes for flight: Attack while flying was already KH1's own flight attack, not a borrow.
> - **t648, flight Attack:** KH1 has no flight attack animation of its own. In flight its Attack step runs three flight rows that use the air combo's own swings and finisher, plus a flight step that keeps him moving at his flight speed and puts him back in flight after. So at Aerial Base Combo KH2, KH2's air combo pick is now made in flight exactly as in a jump, and plays on KH1's flight row with KH2's animation and attack data. KH1's flight case still moves him and returns him to flight. At Aerial Base Combo KH1 it's KH1's own flight attack, unchanged. Checked: the KH1 decomp (Attack step, air picker, row runner) and btltbl's rows. Not checked: in game, and whether our KH2 animations resolve in Neverland's own motion set while flying.
> - **t649, magic where KH1 casts nothing:** KH1 casts only standing / walking (and in Atlantica), jumping / falling, and flying; anywhere else its cast step does nothing and costs no MP. KH1's Gravity, Stop and Aero with their animation set to KH2 were still borrowed by KH2's controller there. Now they follow KH1's rule whatever the animation, as you said: nothing is cast, same as KH1. Checked in the KH1 decomp.
> - **KH2 spells there aren't built yet**, because I hit a conflict. Going by our port of KH2's controller, KH2 itself doesn't cast while gliding, during Quick Run, the dodge roll, the guard or the landing, and it has no ladders, poles, bars or slider. So "1:1 with KH2" in those states would be a refusal, not a cast. That rule came from the Epic exe and I haven't re-checked it on the Steam KH2 exe yet.
> - **3, noted:** missing KH1 systems always get built into KH1's engine, and as little of our code as possible should run every frame, with the assets built once from your KH1 and KH2 files into the mod folder. I've added that to the design instructions (`docs/design-philosophy-instructions.md`). The asset work itself isn't started; I'd plan it with you step by step.
>
> Please test:
> - In Neverland at Aerial Base Combo KH2: attack while flying, a full air combo, and an attack locked on a flying enemy. Look for KH2's swings, damage and finisher, and whether he stays in flight after. Log line: "flight Attack: KH2's … on KH1's flight row …".
> - Gravity / Stop / Aero with their animation at KH2 while gliding or in Quick Run: nothing should be cast and no MP spent. Log line: "KH1's rule, no borrow".
>
> Questions:
> 1. KH2 spells (Magic KH2, Reflect) in states where KH1 casts nothing: (a) refuse them the way KH2 does in the states KH2 has (glide, Quick Run, roll, guard, landing) and the way KH1 does in KH1-only states (ladders, poles, slider), after I re-check KH2's rule on the Steam exe; (b) let them end the glide / Quick Run into KH1's air or ground cast (not what KH2 does); or (c) something else?
> 2. First step for the assets: (a) save the KH2 motion slots we bake at every start (our KH1-format copies of KH2's motions) into the mod folder, built from your own KH1 and KH2 files and rebuilt only when those change; or (b) start somewhere else (rows and attack data, or effects)?

^m7


1. (a)
2. (a) for the transition to real files we need to make sure that the player doesn't need anything to be installed in order to be converted, when the game starts there should be a loading screen showing what the extraction is doing before continuing into the menu, this should render in engine and be native, and not as a layer like the debug menu.

go ahead

> [!tip] Assistant
> Here's this round.
>
> - **t654, KH2 spells in states where KH1 casts nothing.** I re-checked KH2's rule on the Steam KH2 decomp and Sora's motion files. KH2 takes a spell only when the motion playing has a "free" range, or in a combo / Finishing Plus window (`0x1403D6CA0`).
>   - **Quick Run:** its motions have no free range at all, so KH2 never casts during one. Now refused, with KH1's "can't" sound.
>   - **Glide: the old rule was wrong.** KH2's glide motions are free all the way through, so KH2 *does* cast out of a glide. KH1's glide state can't cast, so for now a KH2 spell there is refused with the "can't" sound. See question 1.
>   - **Roll, guard, landing, Aerial Dodge:** KH2 refuses spells until each motion's free frame (roll 36, guard 52, landing 12, Aerial Dodge 4) and takes them after. These run in KH1 states that do cast, and KH1's "can act" opens at KH2's free frame on our slots, so they should already match. Not seen in game.
>   - **Ladders, ledges, poles, swimming, the slider:** Fire to Cure go to KH1's own step, which casts nothing, the same as KH1. Reflect does nothing there.
>   - KH2's controller isn't borrowed in any of these any more. Log: "refused: KH2's spell … no borrow".
> - **t655, the motion cache.** The KH2 motions we bake into KH1 motions at every start are now saved in `kh1-kh2-cache\kh2-motion-slots.bin` in the mod's folder (next to the DLL).
>   - It's built from your own KH1 and KH2 files, with nothing to install.
>   - It's rebuilt only when one of those files changes, when the mod updates, or when one of the ini keys `kh1_slots`, `kh1_leg_ik` or `keyblade_turn` changes. Otherwise it's loaded instead of baked. A damaged file is just rebuilt.
> - **The loading screen isn't built yet.** I found where KH1 can be held just before it loads its title, with its frames still running: its mode state machine (state 2, just before `dktitle.rel` loads). I also found KH1's own text call, the one it uses for its own "loading" message (`0x1402D02F0`). The plan is to draw the progress lines with that same call, in KH1's font. The details are in `docs/kh1-asset-cache.md` §2. I didn't build it this round because it touches the game's start-up, and a mistake there could stop the game starting for everyone; I'd like your OK on the details first (question 2).
>
> Checked: KH2's command rule in the Steam KH2 decomp, the motion ranges in `P_EX100.mset`, KH1's cast step and start-up in the Steam KH1 decomp, the bytes in the exe. Not checked: in game.
>
> Please test:
> - A spell at Magic KH2, and Reflect, during Quick Run and during a glide: the "can't" sound and nothing cast. On a ladder or a pole: nothing happens, no sound.
> - Start the game twice. The first start should log "slot cache: … none yet" and "written". The second should log "matches" and "N block(s) from the cache, 0 baked". The baked moves should look the same as before.
>
> Questions:
> 1. Glide: since KH2 casts out of a glide, should I (a) teach KH1's glide to end into its air state and cast there, 1:1 with KH2, or (b) keep it refused?
> 2. The loading screen: (a) go ahead as planned (KH1 held just before its title, our lines in KH1's own font), shown only when something is being rebuilt; (b) the same, but shown briefly on every start; or (c) not yet.

^m8


for what your working on with glide and the command menu, make it a separate thread 

1. (a) then make it a config in the tweaks config menu
2. (a)
   
   go ahead
> [!tip] Assistant
> Thanks! Two versions this round:
>
> - **t656, casting out of the glide (answer 1).** Moved to its own thread as you asked: [Cast from Glide](../cast-from-glide/cast-from-glide.md). A KH2 spell picked while gliding now ends the glide and is cast in the air, as in KH2, with a Combat > Tweaks row "Cast from Glide" (Off | On, default On) to switch it. Details and what to test are there.
>
> - **t657, the loading screen (answer 2).** When the mod has something to build from your files (the first start, after an update, after a KH1 or KH2 file changes, or after changing `kh1_slots`, `kh1_leg_ik` or `keyblade_turn`), KH1 now waits just before it loads its title. While it waits, it shows what's being built in its own font, on its own frame:
> - "KH2 Combat: building from your KH1 and KH2 files"
> - "Building KH2 move 151 (12 of 96)", then "Saving the built moves"
> - at the bottom: "This happens once, after an update or a change to your files"
>
> The title comes up on the frame the build finishes.
>
> How it's done: KH1's frame runs its game-mode machine first, and in state 2 that machine loads the title (`dktitle.rel`). While we're building, that one call is skipped. The rest of KH1's frame keeps running: window, pad, sound, task list, 2D work and present. The lines are drawn with KH1's own text routine (`0x1402CFBF0`, the one behind its own loading line), right after KH1's 2D work, where KH1 draws its own debug text. Once released, both call sites get KH1's original bytes back, so nothing of ours runs in KH1's frame after that.
>
> It can't get stuck. The hold ends when the loader ends, including when it fails or KH2 isn't installed, and after 90 s at the most (anything still unbuilt then carries on behind the title, as it did before). You can still close the game while it's held.
>
> One thing you'll notice: with a cache that matches, nothing is shown, but the title still waits on a black screen while the mod loads KH2 Sora and checks the cache. In your logs that load takes 0.2 to 0.7 s.
>
> Checked in the Steam KH1 decomp: the mode machine, that nothing else in KH1 reads or times state 2, the quit check and present carrying on, that no movie runs before the title, that the font table is loaded by then, and the text routine's arguments. Checked in the exe: both call sites' bytes and the state address. Not checked: in game; whether the font texture itself is ready by then (if it isn't, the lines would be missing but the hold still works); Epic (Steam only).
>
> Please test:
> - Delete the `kh1-kh2-cache` folder next to the DLL and start the game. You should see the lines counting up, then the title. Log: "loading screen: KH1's title waits for the mod's loader", then "loading screen: title held N ms (F frames, shown); released: the build is done".
> - Start again. No lines, title as usual. Log: "... (nothing shown) ..." with N under a second.
> - Tell me if the screen goes white or freezes instead of black, or if the text is garbled or in the wrong place.
>
> Question:
> 1. On starts where the cache matches, the title waits the fraction of a second it takes to check the cache. Should I (a) keep that wait (it's always this short), (b) not wait, so if a build turns out to be needed after the title has already started, it builds behind the title with no screen, or (c) show a short "Checking your files" line during that wait?

^m9


1. its so fast that I couldn't see anything so (a) is fine
> [!tip] Assistant
> Thanks! (a) it is, so nothing changes: on starts where the cache matches, the title keeps that short wait. No new version this time.
>
> One question passed on from the [RAM use and what's being optimized](../ram-use-and-whats-being-optimized/ram-use-and-whats-being-optimized.md) thread, where you asked me to put it here:
>
> Question:
> 1. After the motion slots (t655), what should be built once into the mod folder next, instead of being built in memory at every start: (a) the KH2 sounds we cut out of KH2's banks (about 9 MB; their cutting is the start-up step that holds KH2's three banks at once, about 28 MB); (b) the KH2 effect packages turned into KH1's format (4 to 12 MB depending on what's unlocked); (c) the Drive Form recoloured textures (this also drops the 10.5 MB of source pixels we keep today); or (d) another order (tell me which)?
>
> The quick RAM fixes you gave the go-ahead for in the RAM thread are going in meanwhile, as their own small versions. They don't depend on this answer.

^m10


update what is still missing for native
> [!tip] Assistant
> Here's the list brought up to date, from t632 to t675.
>
> **Moved to native since the last list**
> - Body hits (Vicinity Break, Explosion, Magnet Burst, Counterguard, Finishing Leap) are KH1 attack objects in KH1's own collision pass (t638). Since t674 every KH2 move re-hits through one path, KH1's own re-arm.
> - Spell hits (Fire, Blizzard, Thunder, Cure, Reflect's counter and burst, Wisdom's shots) are KH1 attack objects made with KH1's own spell constructor (t640).
> - Base Sora's button fallbacks no longer borrow KH2's controller (t639), and neither do spells in states where KH1 casts nothing (t649, t654). Cast from Glide ends KH2's glide into KH1's air cast (t656).
> - Flight Attack: KH2's air picks play on KH1's own flight rows (t648).
> - Idle and landing are baked (t635, t636), so every base-Sora move with a KH1 move to sit on is now a KH1 motion slot.
> - Assets built once into the mod folder: motion slots (t655), sounds (t666), effects (t667), form recolours (t668), with a loading screen in KH1's own frame and font (t657).
> - KH2's knock-back on enemies goes through KH1's launch and flinch (t602 to t605), and now KH1's scripted launch too (t676, in the combo row thread).
>
> **Still running along KH1 (our KH2 logic beside KH1's)**
> 1. The spell scripts still run in our own interpreter. Question 1 in the [magic thread](../kh2-magic-and-reflect-native/kh2-magic-and-reflect-native.md) is still open.
> 2. Reflect's barrier meeting enemy attacks is our own check each frame, not a KH1 guard object.
> 3. Magnet Burst's pull is our own push on the caught enemies.
> 4. Sora's invulnerable frames (roll, Quick Run, Explosion, Aerial Recovery) and the Keyblade guard's facing are decided in our hit-pass hook, not by KH1's own no-damage timer and guard objects.
> 5. KH2's motion-range rules run as our per-frame code, not as KH1 motion events: turn to target, edge stop, the no-gravity hold, rise start, the Keyblade re-arm, guard windows, the Aerial Dodge apex and the glide clock.
> 6. KH2's sounds, voices and effects on baked moves are started by our per-frame tick reading KH2's frame triggers (KH1's engines play them, but KH1's own motion events don't start them).
> 7. Hood, coat and shorts are on t375's rules, applied at bake time; foot IK is off (t634). Both wait for the cloth work in the [sleeves thread](../missing-sleeves-animation/missing-sleeves-animation.md).
>
> **Still on the deprecated KH2 controller or renderer**
> 8. Drive Forms, all of them, and hits taken in a form ([Drive Forms thread](../drive-forms-native/drive-forms-native.md)).
> 9. The fallbacks left for base Sora: a KH2 spell or Reflect whose cast slot didn't build, Attack in the air when the air slots didn't build, and everything on Epic.
> 10. The drawn-over pose, only when a slot fails to bake or a world's motion set lacks the move. Your last log shows it once, for one frame, on a room entry.
> 11. The KH2 player model: our own renderer, off and not loaded (t659) ([model thread](../kh2-player-model-through-kh1s-model-system/kh2-player-model-through-kh1s-model-system.md)).
>
> **Not started:** MP Charge; the Drive gauge, orbs and the other forms; Limits, KH2 summons and reaction commands; KH2's knock-back on Sora himself (KH1's still plays); KH2's ledge hang and climb.
>
> Questions:
> 1. Which next? (a) the spell scripts (1), once you answer the script question in the magic thread; (b) the per-frame rules (5, 6) turned into KH1 motion events that KH1's own motion pass runs, so less of our code runs each frame; (c) Drive Forms (8), starting with the Drive gauge; (d) Sora's own KH2 knock-back as a KH1 state; or (e) MP Charge.
> 2. Invulnerable frames (4): (a) put them on KH1's own no-damage timer, the way KH1's roll does it, or (b) leave them in our hit hook until the moves around them are done?

^m11


1. I want you to implement everything thats not native into being native inside of kh1's engine, if the feature is not in the engine add it to kh1's base engine so that anything from kh2 can work 1:1
   note:  leave out "9. The fallbacks left for base Sora: a KH2 spell or Reflect whose cast slot didn't build, Attack in the air when the air slots didn't build, and everything on Epic.  
2. The drawn-over pose, only when a slot fails to bake or a world's motion set lacks the move. Your last log shows it once, for one frame, on a room entry.  
3. The KH2 player model: our own renderer, off and not loaded (t659)"
   "1. Reflect's barrier meeting enemy attacks is our own check each frame, not a KH1 guard object.
4. Magnet Burst's pull is our own push on the caught enemies.
5. Sora's invulnerable frames (roll, Quick Run, Explosion, Aerial Recovery) and the Keyblade guard's facing are decided in our hit-pass hook, not by KH1's own no-damage timer and guard objects."
6. Hood, coat and shorts are on t375's rules, applied at bake time; foot IK is off (t634). Both wait for the cloth work in the [sleeves thread](app://obsidian.md/missing-sleeves-animation/missing-sleeves-animation.md).
   
7. I want you to transfer drive forms as well
   
   go ahead
   
   
> [!tip] Assistant
> Thanks! This is a big list, so I'm doing it one system per version over the next rounds. Here's how I read your message, then the two versions made this round.
>
> **Leaving out:** 9 (the base-Sora fallbacks and Epic), 10 (the drawn-over pose) and 11 (the KH2 player model).
>
> **Doing, in this order:** 6 (the KH2 sounds and effects on baked moves), 8 (Drive Forms, starting with Valor), 5 (KH2's motion-range rules as KH1 motion events), then 1 (the spell scripts). For the scripts I'm taking your "everything native" as answer (a) to the open question in the magic thread: KH1's own script machine learns KH2's instruction set, and KH2's script calls are added to KH1's table. That's the biggest job, so it comes last. See question 1 about 2, 3, 4 and 7.
>
> **t677, KH2's sound, voice and effect triggers as KH1 motion events (item 6).**
> - When a move is baked, each of its KH2 sound, voice and effect triggers is now written into the KH1 motion's own event list. They sit next to KH1's own "may be left", combo, hit and sound events.
> - KH1's own motion pass fires them as it plays the move. Its sound-event step, `0x140295420`, now knows three new event types and hands them to the KH2 sound and effect players.
> - Our per-frame code no longer reads KH2's trigger list for these moves.
> - **Timing:** each trigger lands on the same frame as in KH2. I checked both games' event passes in the decomps: KH1's fires with `from <= t < to`, KH2's with `from < F <= to`, so ours sit a hair before KH2's frame.
> - **One change you may notice:** a trigger on a move's very first frame now waits until KH1 finishes blending into the move. KH2 does that too.
> - During KH1's own sound-off state these are skipped along with KH1's sounds. I think that state is events and cutscenes, but that's an assumption.
>
> **t678, Valor Form on KH1's own states (item 8, first step).**
> - **The change:** KH1 plays Sora's Drive motion as one of its own requests. It holds him in the air and keeps hits off him with the motion's own frames.
> - **Valor's moves:** after the change, his motion tables switch to Valor's own baked moves: idle, walk, run, jump (High Jump height every time, as KH2 does in Valor), fall, landing, flinches, recoils, item use, and the ground and air combo with Finishing Plus. They carry t677's events.
> - **Combo and physics:** KH1's picker picks from Valor's own combo table. Valor's KH2 numbers run on KH1's states: run 12, jump 235, High Jump MAX 585.
> - **Revert:** switches everything back and drops him into KH1's idle or fall. That covers the menu, running out of MP, and a KO. KH2's controller never takes him.
> - **Not native yet:**
>   - the left Keyblade's hits, which still land from the right blade's position;
>   - the left Keyblade's own drawing, still our extra draw;
>   - Valor's Square moves (the rising slash and the Square finishers): Square does nothing in Valor for now;
>   - Valor's guard windows.
> - **Unchanged:** Wisdom and Final, and the MP cost and drain. The Drive gauge, unlocks and Anti are still waiting on your answers in the [Drive Forms thread](../drive-forms-native/drive-forms-native.md).
>
> **What I checked.**
> - Decomps: KH1's and KH2's event passes and KH1's sound-event step (Steam). KH2's jump start playing 201 with High Jump.
> - Exe: the hooked function's bytes.
> - Files: Valor's combo table, its numbers, its motion list and its ranges, and the Drive and arrival motions.
>
> **What I couldn't check.**
> - Anything in game.
> - Valor's right-hand Keyblade motions file (`W_EX010_BTLF_R` isn't in our extracted files).
> - Whether KH1's "can act" test matches KH2's own rule for when you can change form.
> - The change length: it's 39 frames, the length you asked for earlier, not KH2's full Drive motion.
>
> The first start after this update rebuilds the baked moves once, so you'll see the loading lines.
>
> Please test:
> - **Sounds and effects:**
>   - Explosion, Magnet Burst, Vicinity Break, Aerial Finish and Guard Break;
>   - swing swishes, the Aerial Dodge sound and KH2 casts;
>   - battle quotes at Battle Quotes KH2;
>   - at 30 and at 60 fps.
>
>   Each should start on the right frame, with nothing doubled or missing. Log: "KH1's motion pass fired KH2 motion …".
> - **Valor:**
>   - change on the ground and in the air;
>   - walk, run, jump and fall;
>   - the ground and air combos with finishers and Finishing Plus;
>   - getting hit;
>   - revert from the menu, by running out of MP and by a KO.
>
>   Tell me if a base Sora animation shows anywhere in Valor. Log: "Drive: Valor Form on KH1's own states", "Valor Form's moves on KH1's states: N slots", "Revert (...) on KH1's own states".
>
> Questions:
> 1. Your second quoted block (Reflect's barrier, Magnet Burst's pull, the invulnerable frames) and the hood / coat / shorts line: are those (a) also left out, (b) to be done (I'd put them after the spell scripts), or (c) the hood / coat / shorts left for the sleeves thread and the other three done?
> 2. Footsteps on the baked walk, run and landing still come from KH1's own footstep events, at KH1's step frames scaled onto the KH2 motion. Should they be (a) kept like that, or (b) moved to KH2's own footstep frames, played with KH1's floor sounds?
> 3. Valor next: (a) its Square moves, or (b) the left Keyblade's own hits as a second KH1 attack object?

^m12


1. (b)
2. (a)
3. do both

make sure when your moving these over to be native that you double check everything, we're unsure if we had a 1:1 implementation on our old systems so check the kh2 c decomp steam

go ahead

> [!tip] Assistant
> Thanks! Here's how I took your answers, then this round's two versions.
>
> - **1 (b):** Reflect's barrier, Magnet Burst's pull and the invulnerable frames will be moved too. They go after the spell scripts.
> - **2 (a):** footsteps stay as they are, so nothing changes there.
> - **3 "do both":** done, one version each, below.
>
> The order for the rest stays the same: the other Drive Forms, then KH2's motion-range rules as KH1 motion events, then the spell scripts, then the three from 1. As you asked, each one gets checked again against the KH2 Steam decomp before it moves. This round that check found mistakes in the old versions of both Valor items.
>
> **t679, Valor's Square moves on KH1's own rows.** I went through KH2's Square button handler in the Steam decomp (`0x1403A81F0`) and how it reaches each row (`0x1403A9950`, `0x1403A9C70`). In KH2's Valor:
> - **Square when you're free on the ground** plays the rising double slash (181), standing or moving, with its hop of 90. Valor has no dodge roll or Quick Run, so the "moving" case falls through to it.
> - **Square during a combo** plays 155 on the ground or 183 in the air. They're a normal combo step, not finishers. They only come out in the combo window, with a swing still left and the last swing connected (or Combo Master), and never out of a finisher.
>
> Both run through KH1's own Attack step on Valor's baked slots. KH2's controller isn't involved, and there's no new hook. The old version had two things wrong:
> - it treated 155 / 183 as finishers, so they came out at the combo's end and from finishers;
> - it gave Valor Sora's Finishing Leap, which KH2's Valor doesn't have. That's gone now.
>
> **t680, Valor's left Keyblade hits on its own.** Each of the left Keyblade's hit windows now runs on a KH1 attack object of its own, which KH1's own collision pass checks. It sits on the left Keyblade where it's drawn, with KH2's spheres for that hit, that window's damage and reaction, and KH2's re-hit timing. KH2 keeps each weapon's attacks apart (Steam KH2 `0x1403D25D0`), so both blades can hit the same enemy in one swing. Ours now does the same. The old version had three things wrong:
> - the left blade's hits came out of the right blade;
> - where both blades swing at once (the ground finisher 154, the second air finisher 184), the left hit was lost;
> - a left swing could let the right blade hit the same enemy twice.
>
> Checked: KH2's handler, row checks and attack slots in the Steam KH2 decomp; Valor's combo table, motions, hit windows and weapon spheres in KH2's files; KH1's attack constructor, row maker, collision pass and contact in the Steam KH1 decomp. Not checked: anything in game, and Valor's combo length (KH2 sets it from a script; I used the same length as before).
>
> Assumptions:
> - KH1's "can act" stands for KH2's free state.
> - Combo Master counts as a connected swing.
> - The left spheres use KH2's distances along the blade, the same for every Keyblade.
>
> Please test in Valor:
> - Square standing and while running: the rising slash and its hop.
> - Square after the 1st and 2nd ground swings should give 155, and after it only Finishing Plus can follow. Square at the combo's last swing or during a finisher should do nothing. Do the same in the air with 183, and once after a swing that missed (with and without Combo Master).
> - Ground and air combos with finishers: left-hand swings should hit enemies on the left side, and both blades should land on one enemy where they cross (154, 184).
> - With the debug menu's Combat log on, look for lines starting "Valor: Square" and "left Keyblade hits:".
>
> Questions:
> 1. KH2's spheres for each hit are much bigger than KH1's one Keyblade sphere (155's finisher reaches 380). The right Keyblade still uses KH1's sphere, so should I (a) move the right Keyblade to KH2's spheres too, so both blades are 1:1, or (b) leave it as it is?
> 2. Should the left spheres (a) keep KH2's distances along the blade, or (b) scale to each Keyblade's length, the way KH1's trail does?
> 3. While checking this I found that base Sora's Upper Slash and Horizontal Slash are refused one swing earlier than KH2 allows. Finishing Leap probably comes one swing early too. Should I (a) fix base Sora to KH2's exact rule as its own version, or (b) leave it?

^m13


1. it should fit kingdom hearts 1 in this situation for both keyblades
2. the keyblade should behave exactly like kh1's keyblade so make sure it scales by keyblade
3. (a)

keep working on valor form until its complete, if you have questions for me just tell me after you've already completed the code and we can adjust after. currently I see:

no effects, no proper physics on grounded finisher, attack doesn't interrupt jumps momentum so you can fly high in the air while attacking 

but don't take my word for it compare to how kh2 does it in the c decomp and make sure everything is right. finish valor and wisdom form then we can go from there. make sure when its in the new system that its 1:1 with kh2, our configs don't adjust anything on drive forms atm as we'll treat them as unique in terms of game design. when implementing assume that everything may be wrong on the old controller so double check to make sure its right. it something we've implemented is the same way for drive forms make it apart of that code e.g how magic might work in the air for kh2. if anything is missing from kh1's engine remember, always add that extra ability so that we can achieve 1:1.

for 2nd keyblade it should behave 1:1 with how the main keyblade works, make sure its a file as well 

go ahead

> [!tip] Assistant
> Thanks! I worked through Valor, then moved Wisdom over too. I re-checked everything against the KH2 Steam decomp, and that check found a few more things wrong in the old code. Eleven versions, one system each:
>
> **Your answers 1–3**
> - **t681, base Sora's Square moves by KH2's exact rule (answer 3).** Base Sora now uses the same check as Valor (KH2's `0x1403A81F0` / `0x1403A9950`). With a 3-hit combo, Upper Slash and Horizontal Slash come out after swing 1 or 2. Finishing Leap comes from swing 2 on, tried first, and also in a finisher's window when Finishing Plus leaves another finisher. It used to come one swing early.
> - **t682 and t683, the left Keyblade behaves like the main one (answers 1, 2 and "make sure its a file").** Its hit code is now its own file (`lefthits.rs`). The left hit is now made exactly the way KH1 makes the main Keyblade's (KH1's constructor, row maker and entry build). It uses the same swing data plus the left Keyblade's own weapon stats, and KH1's own code lays its sphere along the left blade. So it scales by Keyblade like the right one, and KH2's bigger spheres are gone. KH2 still decides when each blade's hit is open, its damage and reaction, and its re-hits.
>
> **Valor**
> - **t684, "attack doesn't interrupt the jump's momentum".** KH2 replaces the jump's rise and drift with each air swing's own hop (`0x1403D78D0` → `0x1403CBB00`). Valor was still reading the Base / Aerial Base Combo Config rows: at KH1 the swing played with no hop, so a High Jump kept rising under the attacks. Drive Forms now ignore those rows, as you said.
> - **t685.** When KH2 refuses an Attack press, KH1's own air row with its own hop and lunge used to run under Valor's motion. Now nothing happens, as in KH2.
> - **t686.** KH2 holds you in place through the change **and** the arrival in the air, then you drop from rest. Ours used to fall during the arrival.
> - **t687, "no effects".** The native event path started every effect from base Sora's effect file, and only for the moves in the `kh2_effects` list. Valor's are in its own file, so none played. Also, the Drive and arrival start 13–15 effects on their first frame, and KH1 reads only 8 events at once, so part of the glow was cut. Now each frame's effects are one event, started from the form's own file on its Keyblades, as KH2 does (`0x14040E960`, `0x14040DD70`).
> - **Grounded finisher.** I checked 154 / 155 against KH2's files and code. KH2 moves you only by each move's own root travel, with no extra lunge in Valor's rows, and that's what ours does on level ground. My guess is t684 / t685 were what made it feel off. If it still looks wrong, please describe it or send a log.
>
> **Wisdom**
> - **t688, one path for every form.** Valor's code from t678 is now a per-form table that Wisdom uses too. Valor behaves the same.
> - **t689, Wisdom on KH1's own states.**
>   - Its idle, walk, run, jump, fall, landing, reactions, combo (151 / 154 / 181), arrival and Quick Run are baked slots, with Wisdom's own numbers (run 12, jump 185).
>   - KH2's ground inertia (0.85 / 0.97, `0x140422350` → `0x1403B7AD0`) is now built into KH1's ground step, so it slides to a stop.
>   - Square is Quick Run, per KH2's handler.
> - **t690, its shots.** KH1's own motion events fire them, and they hit as KH1 attack objects in KH1's collision pass. A landed shot chains the combo. **Still running along:** each shot's flight (homing, speed) is our KH2 script runner. The plan to move it onto KH1's own script machine is in the doc.
> - **t691, its magic.** Fire, Blizzard, Thunder and Cure are KH2's, on KH1's cast state with Wisdom's own cast animations and finisher casts. Gravity, Stop and Aero are KH1's spells, played on Wisdom's motions.
>
> Checked: the Steam KH2 and KH1 decomps, and KH2's files (PTYA, prty, msets, effect triggers), as noted per version in AGENTS.md. Not checked: anything in game. The first start bakes once (Valor's effect slots and Wisdom's files).
>
> **Please test**
> - **Valor:**
>   - jump and High Jump, then attack at once: you should stop rising and get a small hop per swing;
>   - air and ground combos with 154 / 155, and Drive in mid-air;
>   - the effects: Drive glow, arrival light, 154, Square moves;
>   - try it with the combo Config rows on KH1 and on KH2: it should be the same;
>   - two Keyblades of different lengths: the left should reach like it does in the right hand.
> - **Base Sora:** Square after swings 1 and 2, on the ground and in the air, with and without Finishing Leap.
> - **Wisdom:**
>   - walk and stop (the slide), and Quick Run standing and moving;
>   - the shot combo on the ground and in the air, and whether hits chain without Combo Master;
>   - each spell on the ground, in the air and mid-combo;
>   - arrival and revert.
>
> **Questions, now that the code's in**
> 1. The left blade now runs KH1's own wall test, so a left swing into a wall bounces like a right one. Keep that (a), or turn it off for the left (b)?
> 2. The second Keyblade's draw and trail are still ours, next to KH1's (a four-step plan is in the doc, §7.3). Next, should I start with (a) KH1's own weapon object, (b) the trail, or (c) the draw?
> 3. On the ground, KH2 plays Valor's short stance motion (6) after the arrival, before the idle. Add it (a), or leave it (b)?
> 4. In KH2, the end of Valor's 154 can carry you off a ledge and hold you in the air. KH1 cuts the swing when you leave the floor. Change KH1's rule (a), or leave it (b)?
> 5. Wisdom's Cure is now KH2's own Cure. The old main branch had KH1's Cure on KH2's animation. Keep KH2's (a), or go back to KH1's (b)?
> 6. Wisdom's Quick Run is Sora's level + 1. KH2 uses the form's own level, which KH1 doesn't have. Keep it (a), or build form levels into KH1 (b)?
> 7. In KH2's code a missed swing doesn't count as connected, so Square after a missed air swing is now refused. Our Attack button still lets missed air swings chain. Should I trace that next (a), or leave it (b)?
> 8. Base Sora, combo rows on KH2: when KH2 refuses an Attack press, KH1's own row still runs. Make that "nothing" too, as in KH2 (a), or leave it (b)?
> 9. KH2 also has a frame trigger on the Drive and Valor's recoil that looks like pad rumble. Add it (a), or skip it (b)?
>
> Next I'll keep going on Valor and Wisdom by your answers. Then Wisdom's Aerial Recovery (not built yet) and the shots' flight onto KH1's own script machine.

^m14


1. (a)
2. all three, make it native to kh1 like the base keyblade 
3. (a)
4. (a) and can you make it in the debug
5. (a)
6. (b)
7. (a)
8. if your running another instance of the combo row natively that still isn't align with 100 percent native, so I want you to patch the features kh1's combo row and other related logic are missing (like we've been doing with base kh2 soras abilities) so that valor and wisdom can behave correctly without overwriting kh1's vanilla logic. 
9. (a) if rumble is on it should work then

go ahead

> [!tip] Assistant
> Thanks! All nine are in. 1 (a) and 5 (a) need no change: the left blade keeps KH1's wall bounce, and Wisdom keeps KH2's Cure. The rest is nine versions, one system each. I re-checked each one against the KH2 Steam decomp first.
>
> **7, Attack after a missed swing**
> - **t692.** KH2's Attack button (`0x1403D6CA0`) has no separate air rule. In a swing's combo or finisher window the press only counts if that swing connected, or with Combo Master. Otherwise nothing happens, and once the swing ends the next press starts a new combo. So a whiffed air combo never reaches its finisher.
> - KH1's own Attack step already works exactly this way, so base Sora, Valor and Wisdom on KH1's states were right.
> - The old KH2 controller code still used by Master, Final, Limit and Anti let missed air swings chain. That's fixed now.
> - I also confirmed which of KH2's ability bits are Combo Master and Berserk Charge (both were guesses before).
>
> **8, KH1's combo row refusing on its own**
> - **t693, first step.** When KH2 takes nothing for a press, KH1's own row picker now answers "no attack" itself, and the row runner runs nothing. That's one rule for base Sora at the KH2 combo settings and for Valor and Wisdom. Before, base Sora played KH1's own row there, and the forms used a separate check afterwards.
> - A refused press also clears KH1's "connected" flag, as KH2's does. With the combo rows at KH1, nothing changes.
> - **Still running along:** KH2's row choice is still worked out on our KH2 controller object, fed with KH1's state, and the Finishing Plus counter is ours. Next steps: the row choice from KH1's state alone (Square moves and Counterguard move with it), then the finisher counter as a field of KH1's own combat info.
>
> **3, the stance after the change**
> - **t694.** KH2 plays the arrival, then motion 6, then the idle, every time a form starts on the ground (`0x140405630`), for every form. Motion 6 is free all through, so moving, jumping or attacking cuts it.
> - It's now a baked slot for each form that has one: Valor (half a second) and Wisdom (two thirds of a second). KH1's own motion queue plays it, with KH2's blends. In the air nothing changes.
>
> **9, rumble**
> - **t695.** KH2's trigger 31 is the pad vibration. It's on the hit reactions, recoils and guard repel, the Drive, the casts, Explosion, Finishing Leap, Counterguard, Magnet Burst and Wisdom's shots.
> - These now fire from KH1's own motion events and play KH2's exact patterns (from KH2's battle file) through KH1's own vibration player.
> - They only rumble when KH1's Config **Vibration** is on.
>
> **4, ledges**
> - **t696.** KH2 only cuts a ground move into the fall when you're off the floor *and* the move isn't holding you (its no-gravity or edge-stop frames).
> - KH1's cut is otherwise already KH2's (fall from rest, speed capped at 8). So KH1's ground state now skips the cut while one of our moves holds you.
> - Valor's 154 holds you all the way through: it carries you off a ledge and you drop when it ends. Valor's 155 holds too, and so do base Sora's 154 / 155 / Upper Slash / Guard Break / Flash Step during their edge-stop frames, and the Drive.
> - KH1's own moves are unchanged.
> - It's in the debug menu's main page as **Ledge Carry** On/Off, on by default and saved in the ini.
>
> **2, the second Keyblade as one of KH1's own weapons**
> - **t697, weapon object:** it's now taken from KH1's own weapon pool and built the way KH1 builds Sora's.
> - **t698, drawing:** KH1's own character draw, the same function that draws Sora and his Keyblade, now draws it.
> - **t699, trail:** KH1's own trail code makes, fades and ends it. The left blade's trail windows are now a motion event in Valor's moves, read by KH1's own event reader. Vanilla KH1 never reads that event.
> - The left hits use the new weapon object.
> - **Still ours:** the "holder". KH1 gives each character one weapon on one set of hand bones, so the left blade needs a second copy of Sora that carries the left hand's bones (taken from KH2's left Keyblade). That's the last step of the plan; see question 3.
>
> **6, form levels**
> - **t700.** Drive Form levels are in KH1 now, by KH2's own rule (`0x1403E8380`), with the EXP per level read from your own KH2 files.
> - **Valor** gets 1 EXP per hit it lands on an enemy (both Keyblades count).
> - **Wisdom** gets 1 per enemy you defeat after hitting it last.
> - Each form's growth ability now follows its level, as in KH2: Valor's High Jump, Wisdom's Quick Run, Final's Glide. It's LV1 at form LV1, LV2 at LV3, LV3 at LV5 and MAX at LV7. This replaces the old "MAX" and "Sora's level + 1" rules.
> - The levels are kept in three spare bytes of your save. A "Form Level Up" box shows each new level.
>
> **What I checked**
> - The Steam KH2 and KH1 decomps for everything above.
> - The exe: every new hook's first bytes, and KH2's growth table.
> - KH2's files: motion ranges and triggers, the vibration table, fmlv.
>
> **What I couldn't check**
> - Anything in game.
> - Which pad motor KH2's two vibration channels drive.
> - That KH1's blend counts 60 Hz frames.
> - That nothing in KH1 touches the new save bytes through a computed pointer.
>
> The first start rebakes once (the new stance slots, the rumble events and Valor's left trail windows).
>
> **Please test**
> - **Missed swings** (base Sora at KH2, Valor, Wisdom):
>   - whiff on the ground and in the air, with and without Combo Master: missed swings shouldn't chain, and air whiffs never reach the finisher;
>   - mash Attack where KH2 does nothing (after an air combo, during a finisher without Finishing Plus): nothing should come out;
>   - Square moves and Counterguard should still work.
> - **Stance:** Drive into Valor and Wisdom on the ground. You should get the stance, then the idle. Move, jump or attack during it.
> - **Rumble:** with Vibration on, get hit, get repelled, Drive, cast, and use Explosion / Magnet Burst. Then turn Vibration off: nothing should rumble. Try 60 and 30 fps.
> - **Ledges:** start Valor's 154 near a ledge. You should be carried off and drop when it ends. With Ledge Carry off you drop at once. Base Sora's finishers should still stop at edges.
> - **Left Keyblade:**
>   - Does it sit in the hand as before?
>   - Is its trail on the left blade, in its own look, fading at the swing's end and on revert?
>   - Change rooms and swap Valor's Keyblade while in Valor.
> - **Form levels:** land hits in Valor and defeat enemies in Wisdom. Watch the level-up box, and check High Jump / Quick Run improve at form LV3. Save, reload and check the level is kept.
> - **Log lines:**
>   - "KH2's walk takes no row"
>   - "its arrival, then its stance (motion 6)"
>   - "KH2 rumble"
>   - "KH2's ledge carry"
>   - "its KH1 weapon object made"
>   - "drawn in the left hand by KH1's actor draw"
>   - "made by KH1's trail step"
>   - "Valor Form (LVn, x EXP)"
>
> **Questions**
> 1. In KH2 a clash (and two other cases, probably wall bounces) counts as a connected swing but takes one off the combo count, so the next press repeats that step. Should I (a) add that to KH1's clash / wall handling, or (b) leave it?
> 2. KH2's magic always counts as a combo step, even after a missed swing. Our older controller code (Master, Final, Limit, Anti) still counts a ground spell only after a hit. Should I (a) fix it to KH2's rule, or (b) leave it?
> 3. The second Keyblade's last step, the holder: should I (a) bake the left hand's weapon bones into Valor's moves so KH1's own animation drives them, or (b) keep using KH2's left Keyblade pose for now?
> 4. Final's left trail still comes from KH2's data, because Final isn't on KH1's states yet. Should I move it over (a) when Final goes native, or (b) now?
> 5. Form level cap. KH2 caps form levels at 2 + the forms you own. Here that's Valor and Wisdom (plus Final once you've used it), so LV4, or LV5 with Final. Should I (a) keep KH2's rule, (b) count every KH2 form so the cap is LV7, or (c) wait until Limit and Master are on these states?
> 6. Base Sora's growth from form levels. In KH2 a form level-up also raises base Sora's own growth ability and gives him that level's reward ability (Auto Valor, Combo Plus, MP Rage, ...). That would change KH1's own ability list. Should I (a) leave it out, (b) add the growth only, or (c) add both?
> 7. Final's EXP. Final levels on defeating Nobodies, and KH1 has none. Should it (a) never gain EXP, (b) count every enemy defeated in Final, or (c) use another rule you pick?
> 8. KH2's form level also lengthens the form's gauge. Here forms run on MP. Should higher levels (a) leave the MP drain as it is, or (b) drain more slowly?
> 9. Rumble on hits. KH1's own hit vibration still starts the moment you're hit, then KH2's reaction vibration replaces it a moment later. KH2 has no separate hit vibration. Should I (a) stop KH1's hit vibration when a KH2 reaction will play its own, or (b) leave both?
> 10. While you're carried off a ledge, KH1's ground checks (pushes, grabs) still run. Should the carry (a) block ladder and ledge grabs until it ends, or (b) let KH1 grab as it does now?

^m15

