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


