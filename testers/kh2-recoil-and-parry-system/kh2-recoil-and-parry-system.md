# KH2 recoil and parry system

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from Sora's hit reactions native(../so](#^m1)

---


> [!tip] Assistant
> I started this thread from [Sora's hit reactions native](../soras-hit-reactions-native/soras-hit-reactions-native.md), as you asked. This first message is the research; nothing has changed in the code yet. The notes are in `docs/kh1-kh2-recoil.md`.
>
> **How KH2 does it** (KH2 Steam decomp and your KH2 files):
> - **Whether two attacks meet.** A 7×7 table by attack kind decides it (normal, pierce armor, guard, special guard), then each attack's reflect angle is checked (`0x1403D08E0`).
> - **What a meeting does.** Both attacks stop, unless one's "refact self" is 2. Each side also becomes immune to the other, so the enemy's hit can't land.
> - **Sora:** on the ground he recoils (motion 41, pushed back 50); in the air it's motion 43 with a hop of 50 (`0x1403D61D0`). If it was against a guard with refact 1, he does the guard repel (42) instead, followed by 80 frames of no damage.
> - **The enemy:** it plays its "reflected motion", pushed back by its reflect distance. That's the parry.
> - **The combo:** either recoil takes one swing off his combo. KH1 does the same.
>
> Guard Break's hit is pierce armor with refact self 2. It goes straight through normal attacks and plain guards, only meets special guards, and nothing ever stops it.
>
> **How KH1 does it.** KH1 decides by priority, not by kind:
> - Two attacks meet only if one of them is under 0x62.
> - A guard (priority 100) is never hurt.
> - The weaker side recoils: kind 6, or kind 7 (KH1's big recoil) when the other side is more than 40 stronger.
> - A side at 0x62 or more is never stopped.
> - Sora's recoils are KH1's own animations, with a push from the priority gap, and 8 frames of no damage.
>
> **Why you can't parry:**
> - **A bug in the borrowed moves.** In moves where KH2's controller still borrows Sora, the code treats his body as the Keyblade. So the Keyblade's swing never meets an enemy's melee attack, only thrown things. Your old logs show only "a shot" meetings.
> - **KH1's rule in KH1's own states.** The parry does happen there, but by KH1's priority rule with KH1's recoil animations, and against any enemy attack of priority 0x62 or more it can't happen at all.
> - **Guard Break** passes guards like KH2's does. But it still meets enemy attacks and knocks the enemy into KH1's big recoil, which KH2's wouldn't do.
> - **No protection after the repel.** Our guard repel doesn't have KH2's 80 frames of no damage yet.
>
> **The plan, one system per version, all through KH1's own clash:**
> 1. Fix the Keyblade/body mix-up, so KH2's swings meet enemy attacks through KH1's own meeting.
> 2. Put KH2's recoil on KH1's recoil requests: 41 on the ground and 43 in the air, with KH2's push of 50 and KH2's hop. KH1's combo count goes down by one, the same in both games.
> 3. Put KH2's clash rules on our attack data:
>    - Guard Break and Counterguard become a type that meets no attacks or guards but still hits bodies.
>    - Moves with refact self 2 get a priority KH1 can't stop.
> 4. Give the guard repel its 80 frames of no damage, through KH1's own timer.
> 5. Explosion, Vicinity Break and Magnet Burst: when their body hits meet a guard or an attack, Sora recoils on KH1's states. Right now he doesn't.
>
> Questions:
> 1. KH1's big recoil (when the enemy's attack is much stronger) has no KH2 version. Should it be (a) KH2's normal recoil, (b) KH1's big one, or (c) a Config row?
> 2. Guard Break only meets KH2's "special guards", and KH1's guards don't have that kind. Should it (a) meet no KH1 guard at all, (b) treat KH1's directional guards (shields that only cover the front) as special guards, or (c) go enemy by enemy?
> 3. When a move that can't be stopped meets an enemy attack, KH1 gives the enemy its big recoil, where KH2 plays the enemy's normal one. Should we (a) keep KH1's big recoil, (b) lower our priority so it's the normal one (then KH1 can sometimes stop the move), or (c) catch that one case and write the normal recoil ourselves?
> 4. After an attack clash, KH1 gives Sora 8 frames where he can't be hurt; KH2 gives none. Should it be (a) KH2's none, (b) KH1's 8, or (c) part of the knock-back switch from the hit reactions thread?
>
> It would help to have a log and a Select + Triangle trace of you swinging into an enemy's attack: once with a plain swing and once with Guard Break, so I can see which case you're running into.

^m1
1. (b)
2. unsure what kh2s special guard is or when its used so please let me know, for now do (c)
3. (a)
4. should be part of a recoil switch in the config menu, if you switch to kh1 all moves including kh2 uses kh1's rules exactly and vice versa
go ahead
