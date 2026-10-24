# Dodge Roll, Guard and Counterguard native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

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


