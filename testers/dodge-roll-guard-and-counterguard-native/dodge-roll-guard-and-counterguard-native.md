# Dodge Roll, Guard and Counterguard native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Thanks. Here's how I read each answer, then one go-ahead to ](#^m2)
- [3. Assistant: Done, one version each:](#^m3)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. Research only so far, no code.
>
> **What we have now.** With Dodge Roll at KH2, Square on the ground hands Sora to KH2's controller for its roll, guard, the guard's repel and Counterguard. At KH1, KH1's own roll runs with KH1's animation.
>
> **What KH2 does** (KH2 Steam decomp). Its action button handler (`0x1403A81F0`) tries the Square combo moves first. Then it guards, but only with the stick at rest (`0x1403A9C70`: row 11, motion 173, facing the target, speed zeroed). Next comes the dodge roll, then Quick Run. The guard's window is frames 6..37. A block plays the repel, motion 42. Attack while 42 plays is Counterguard (row 39, motion 171), checked in the attack handler `0x1403A76B0`. The roll's invulnerability is 10 / 16 / 24 / 32 frames by level.
>
> **What KH1 does** (KH1 Steam decomp):
> - Square in ground state 0 goes to `0x1402A7B30`. It guards when Guard is equipped and either there's no Dodge Roll or the stick is only lightly tilted (a threshold from his stats). Otherwise it rolls (`0x1402C9E60`, motion 0xDC).
> - KH1 has **no guard state**. Its guard is attack row 0x3D (motion request 0xD4): a guard attack object, priority 100, a pure guard, about +-100 degrees around his facing. That's the same kind of object we already make for KH2's guard windows.
> - KH1's Counterattack swaps the menu's Attack for command 99 (row 0x3E). That happens while Sora plays his clash recoil 0x6E after a clash, with the ability on and an enemy targeted.
>
> **What I couldn't find yet:**
> - What puts Sora into 0x6E after his *guard* blocks a hit (the clash itself gives the guard side nothing).
> - Where KH1's roll gets its invulnerable flag (`0x40`, set by op 0x2B in `0x14028E6E0`).
>
> A short trace would settle the first: guard, take a hit, then Counterattack.
>
> **The plan, one system per version:**
> 1. **Roll.** KH1's own roll at Dodge Roll KH2 too, drawn with KH2's 209 as a KH1 slot. It gets KH2's travel, KH2's invulnerable frames by level and KH2's free frame (34). Roll vs guard follows KH2's rule (stick at rest = guard).
> 2. **Guard.** KH1's guard row 0x3D with KH2's 173 in its slot. Its guard object is live on KH2's frames 6..37.
> 3. **Repel and Counterguard.** KH2's 42 on a request of our own; KH1's 0x6E is its swing-clash recoil, which KH2 draws as 41. Attack during it runs a KH1 row for Counterguard (171).
>
> Each step drops the borrow for that move. KH1's guard tally (an event counter) keeps counting.
>
> Notes, with addresses and how sure each one is: `docs/kh1-roll-guard-native.md`.
>
> **Update while I wrote this:** t489 (from the combo row selection thread) has already put KH2's guard (motion 173), its repel (42) and Counterguard on KH1's states as requests of their own, so plan steps 2 and 3 are covered there. What's left here is the native dodge roll, and the questions below about guard coverage and KH1's Counterattack command still apply to how t489 behaves.
>
> Questions:
> 1. Guard's coverage: (a) KH2's, front only unless Guard Plus is on; (b) KH1's guard data, about +-100 degrees.
> 2. While the repel plays, should the command menu show KH1's Counterattack command in Attack's place? (a) yes, as KH1 does; (b) no, Attack just does Counterguard, as KH2 does.
> 3. Once the roll is native, the Dodge Roll style row at KH1: (a) keep it (KH1's own roll animation and numbers); (b) take it out.

^m1


1. unsure go with whats most native and 1:1
2. (a)
3. if you can make dodge roll its own slot and just have it get behaviours ported from kh2 so its 1:1 then we just switch which dodge roll out with the config use kh1s as a base so its native. Your call on what would be the best but it seems as a user to me reading it that this is the cleanest way, but tell me what you think. 


> [!tip] Assistant
> Thanks. Here's how I read each answer, then one go-ahead to ask for.
>
> 1. Most native and 1:1 means (a), KH2's coverage: front only, all round with Guard Plus. It runs on KH1's own guard object, though, not KH1's ±100° guard data. That's what t489 already does for the guard press: its Keyblade guard is a KH1 guard object, with the front check from KH2's `refact_self`. I'll double-check that t489's front test is KH2's angle and that Guard Plus widens it. If either is off, it gets its own version.
> 2. (a): while the repel (42) plays, the menu's Attack slot shows KH1's Counterattack command, as KH1 swaps it in during its clash recoil. One thing to flag: KH2 has no such menu change (Attack just does Counterguard there). So this is KH1's look on top of KH2's move. Choosing it would run Counterguard (row 39 / motion 171, our request 0x20A), not KH1's own counterattack row 0x3E. Pressing Attack without the menu still works the same.
> 3. That's what I'd do too, and it's the plan's step 1. **KH1's roll is the base at both settings:** Square starts KH1's own roll (its state and its request 0xDC), so there's never a borrow. The Dodge Roll row only picks what plays on that roll:
>    - **At KH2:** the request plays our slot of KH2's motion 209, with KH2's travel, its invulnerable frames by level (10 / 16 / 24 / 32), its free frame (34), and KH2's guard-or-roll rule (stick at rest = guard).
>    - **At KH1:** KH1's own animation and numbers, as now.
>
>    So the row stays, and it's one roll with two sets of behaviour.
>
> Things this changes or leaves:
> - KH1 only rolls with its own Dodge Roll ability on. At KH2 I'd start KH1's roll directly when KH2's Dodge Roll is equipped, so the ability check is KH2's.
> - Quick Run out of a held roll stays as it is for now (it's the Quick Run thread's).
> - KH1's roll invulnerability (flag `0x40`, which I still haven't traced to where KH1 sets it) would be replaced at KH2 by KH2's frames on our slot, the way the other slots do it (`kh2slots::invulnerable` → `hurt`).
>
> If that all sounds right, write "go ahead" and I'll build: the roll first, then the menu swap (2), then the guard coverage check (1) if it needs a fix. One version each.

^m2


go ahead

> [!tip] Assistant
> Done, one version each:
>
> - **t523, the roll.** With Dodge Roll at KH2, Square while moving now starts KH1's own roll (no borrow). Its request plays KH2's roll (motion 209) in KH1's roll slot. KH1's roll request already moves Sora by the animation's root, so he travels KH2's distance. He's invulnerable for KH2's frames by level (10 / 16 / 24 / 32), and KH1's own 12 frames of no damage are taken off. You can move, jump, roll or attack out of it from KH2's frame 34 (KH1's roll lets go at 30). Square with the stick at rest is still the guard (t489), and Square held through the roll still turns into Quick Run at KH2's check frame. At KH1 the roll is KH1's own, as before. The borrow is only a fallback now (Drive Forms, or the slot not built).
> - **t524, the menu.** While the repel (42) plays and Counterguard is equipped, the command menu's Attack shows KH1's Counterattack command. Choosing it does Counterguard (171), not KH1's own counterattack. It doesn't need a target, because KH2's Counterguard doesn't (KH1's own swap wants a locked enemy).
> - **t525, guard coverage.** I checked it against KH2's code, and t489's was off. KH2 has **no Guard Plus**: that flag was the old rewrite's own and nothing ever turned it on. KH2's real rule is each guard's own ATKP angle. The guard press (161) holds 120° either side of his facing, not 90°. The repel's own guard (162) holds every side, where before it only held the front. The body guards (Aerial Dodge, Aerial Recovery) were already all round, and they still are. This applies to the hit check and to the KH1 guard objects alike.
>
> **How I checked:**
> - KH1's roll start, its 12-frame timer and the Counterattack swap come from the KH1 Steam decomp.
> - KH1's roll request (it has KH1's travel flag) and its slot (38 frames, let go at 30) come from KH1's motion file.
> - KH2's guard angle test (`0x1403D1F40`) comes from the KH2 Steam decomp, and the guard ATKPs' angles from `00battle.bin`.
>
> **What I couldn't check:**
> - Where KH1 gets its roll's untouchable flag. It isn't in the roll's animation, and it doesn't matter at KH2 now.
> - That nothing else in KH1 reads the roll's timer bit as "rolling".
> - Which way KH1 turns a guard arc's angle. It's unused, since every angle of Sora's is 0.
>
> **Please test:**
> 1. Roll at each Dodge Roll level. Check the distance, getting hit in the first frames (and just after them), and walking, jumping or attacking out from frame 34.
> 2. Roll held into Quick Run.
> 3. Guard a hit from the side (about 100°), and get hit from behind during the repel.
> 4. During the repel, check that Attack shows Counterattack in the menu and that choosing it does Counterguard.
> 5. A trace of a roll would help (Select + Triangle).
>
> **Questions:**
> 1. Should the menu's Counterattack also need a locked-on enemy, as in KH1? (a) no, show it whenever Attack would do Counterguard (what t524 does); (b) yes, only with an enemy locked, as KH1 does.
> 2. KH1's roll has its own floor sounds (thumps by surface), and I kept them under KH2's roll, the same way the swings keep KH1's. (a) keep them; (b) KH2's own roll sounds only.

^m3

