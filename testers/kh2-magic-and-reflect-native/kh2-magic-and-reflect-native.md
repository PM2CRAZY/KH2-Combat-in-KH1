# KH2 magic and Reflect native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: start working on "- KH2 magic and Reflect: a borrow, plus ou](#^m1)
- [2. Assistant: Moved here from "What is still missing", so that thread stay](#^m2)
- [3. Assistant: t477 and t478. Started on native magic.](#^m3)
- [4. Assistant: t484 to t487. The first batch from your answers, one system ](#^m4)
- [5. Assistant: Thanks, that clears up Cure. With the Cure row at KH1 you'll](#^m5)

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


1.(a) 
2.(a) (with the exception to drive forms for now)
3.if b doesn't cost a lot of computational cost then do it, if not a
4.(c)
5.make a menu in the magic config to change what gets casted kh1 cure or kh2
6.(b)

go ahead


> [!tip] Assistant
> **t484 to t487.** The first batch from your answers, one system per version:
>
> - **t484, MP (6b).** Base Sora's KH2 spells now cost KH2's share of his MP: Fire 12%, Blizzard 15%, Thunder 18%, Reflect 10%, and Cure takes all of it. It's measured against KH1's max MP and paid through KH1's own MP payment, so the yellow charge part goes first, then the bars. Drive Forms still cost 1 MP (your exception). Checked: the costs in KH2's files, KH1's payment and MP check in the decomp. One gap: KH1's menu still greys a spell by KH1's own cost, so a spell you can afford at KH2's price can still show grey.
> - **t485, Cure row (5).** Config > KH1 in KH2 > Combat > Magic has a Cure row (ini `cure_style`, KH2 by default). KH2 casts KH2's Cure. KH1 casts KH1's own Cure on KH2's Cure animation, the way Gravity, Stop and Aero work with Magic on KH2. The row only matters with Magic set to KH2.
> - **t486, the combo (1a).** A cast now counts as a step of KH1's combo and the combo carries on after it, on KH2's timing from the files:
>   - From KH2's reaction window (Fire 30, Blizzard 24, Thunder/Cure 26, air casts 36 to 44) an Attack goes on with the combo, and jump or Square cut the cast. The stick alone doesn't, as in KH2.
>   - Another spell can follow only inside KH2's magic window: Fire 34 to 46 and from 60, Thunder/Cure from 42, never for Blizzard or the air casts.
>   - A finisher cast ends the combo. Finishing Plus after a finisher cast isn't done yet.
>   Checked KH1's Attack step in the decomp.
> - **t487, air casts (2a, physics KH2, Drive Forms unchanged).** An air cast stops him as KH2's does: no sideways speed, no more rise; he drops from rest at KH2's gravity until the cast's hover frames, hovers, then falls. KH1's own air cast kept 70% of his speed and the rise. Ground casts are unchanged (I'm assuming KH1 doesn't move him during them).
> - **Reflect (4c):** kept as it is, still blocking every hit.
> - **Spell hits (3):** not built yet. I went through KH1's hit pass and (b) is cheap: a few extra spheres a frame in KH1's own code. The real limit is KH1's 64-slot attack pool, which enemies share, so it'll be (b) with several spheres sharing one hit, next round. The plan is in `docs/kh1-magic-native.md` §7.
>
> None of this is tested in game. Please test: the MP drain at a few MP levels (Cure with very little MP too), both Cure settings, Attack / jump / Square out of each cast and spell into spell, a cast mid-combo and as the finisher, and an air cast mid-jump.
>
> Questions:
> 1. KH2 Cure's heal amount (your answer 5 picked the menu, not the formula). Right now it heals a share of max HP. Should it be (a) KH2's formula with max MP standing in for Magic, (b) KH1's Cure amount, or (c) left as it is?
> 2. Cure on KH1: (a) KH1's Cure on KH2's Cure animation (as built), or (b) KH1's whole cast, its own animation too?
> 3. Menu greying by KH2's cost: (a) fix it next (a hook on KH1's usable check), or (b) leave it?

^m4


1. and 2. when I meant by cure switch it changes what you cast, cure kh1 would cure exactly like kh1 with its effects and cast while the cure switch for kh2 would switch. 
   
   reflect currently does not behave properly for kh1 physics, please see how casting is done in the air physics wise to get the same for kh1, as for kh2 please check the c steam decomp to fix and find any issues with the physics, it was adjusted on the main branch but its not accurate to how kh2 does reflect in the air. 
   
   
> [!tip] Assistant
> Thanks, that clears up Cure. With the Cure row at KH1 you'll get KH1's whole Cure: its own animation, its effects and KH1's cost. Right now it's KH1's Cure drawn on KH2's Cure animation. With the row at KH2 it's KH2's Cure all the way through, and its heal will be KH2's formula (that was the question 1 left open). As always, the Physics row decides how he moves while casting.
>
> I went back through both decomps for air casts, and my last note was wrong about KH1:
> - **KH1's own air cast** doesn't keep 70% of your speed or your jump's rise. It halves your sideways speed and gives you a fresh 30-unit hop from wherever you are, then KH1's normal fall, with no hover. Touching down cuts the cast into KH1's landing (`0x1402A3600` → `0x1402AF960`; the 30 and the 0.5 are in btltbl).
> - Fire, Blizzard, Thunder and Cure already do exactly that at Physics KH1. **Reflect doesn't**, because it's still cast by KH2's controller: it stops you dead, skips the hop, drifts with KH2's drag and lands in KH2's landing. That's what you're seeing.
> - **KH2's air cast** isn't what we built in t487 either:
>   - KH2 pops you up 40 units (sstm's "magic jump", 40.0) and keeps your sideways momentum, letting it fade by 0.95 a tick (`0x1403C5170` → `0x140409B00` → `0x1403CBC30`).
>   - It then holds you for the cast's hover frames (16 to 74 for Reflect). The arc's clock keeps running during the hover, so once it ends you drop at full fall speed straight away.
>   - Landing during the cast cuts into KH2's landing.
> - **Ours at Physics KH2** stops you, drops you from rest until the hover, starts the fall gently again afterwards, and Reflect keeps playing on the ground if you land.
>
> The full comparison is in `docs/kh1-magic-native.md` §4e.
>
> **The plan, one version each:**
> 1. Cure KH1 as KH1's whole cast.
> 2. Reflect moved onto KH1's own cast state like the other spells, no more KH2 controller. That alone makes it exactly KH1 at Physics KH1.
> 3. KH2's 40 hop and kept momentum at Physics KH2, for every air cast.
> 4. KH2's hover with the clock running, so the fall after it is KH2's.
> 5. Landing mid-cast cuts like KH2.
> 6. Next after that: the menu greying spells by KH1's cost instead of KH2's (question 3 last time; I'll take it as "fix it" unless you say otherwise).
>
> Questions:
> 1. The hover's running clock: (a) only for spells, (b) also for KH2's moves with the same kind of hover (Aerial Dive, Finishing Leap), whose falls would change too, or (c) leave hovers as they are?
> 2. Landing in the middle of an air cast at Physics KH2: (a) KH2's landing animation, (b) KH1's landing, or (c) follow the Misc Animations row?
>
> Say "go ahead" with your answers and I'll start with Cure and Reflect.

^m5

1. (b) unless you have kh1 physics on
2. the jump config controls what landing animation you get currently so follow that
   
   go ahead
