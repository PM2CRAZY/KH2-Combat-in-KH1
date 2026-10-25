# KH2 recoil and parry system

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from Sora's hit reactions native(../so](#^m1)
- [2. Assistant: Done, one system per version:](#^m2)

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

> [!tip] Assistant
> Done, one system per version:
>
> - **t540**: the bug that stopped parries in the borrowed moves. The code took KH2's weapon 0 (his body) for the Keyblade, so Keyblade swings only ever met thrown things. Now the Keyblade swing meets enemy melee attacks through KH1's own clash.
> - **t541**: a new **Recoil** row in Config > KH1 in KH2 > Combat (KH1 by default).
>   - **At KH1**, everything is KH1's: priorities, KH1's recoil animations, its push and its 8 frames where he can't be hurt.
>   - **At KH2**, KH1 still runs the recoil (combo count, HUD and so on), but Sora plays KH2's recoil: motion 41 on the ground with KH2's push of 50, and 43 in the air with KH2's hop. He gets no protection afterwards.
>   - KH1's big recoil keeps KH1's own animation, as you picked (1b).
> - **t542**: KH2's clash rules on our attacks, at Recoil KH2:
>   - **Pierce armor** (Guard Break, Counterguard, Finishing Leap, Explosion, Vicinity Break, Magnet Burst) meets no enemy attack and no guard, and is never stopped.
>   - **"Refact self 2" moves** (Horizontal Slash, Retaliating Slash, Slapshot, Dodge Slash, Flash Step, Aerial Sweep, Aerial Finish, Aerial Dive) meet attacks but are never stopped. The enemy's attack is stopped and the enemy gets KH1's big recoil when the gap is big, as you picked (3a).
>   - At KH1 every move uses KH1's priorities, Guard Break included.
> - **t543**: KH2's guard repel now gives KH2's 80 frames of no damage (Recoil KH2), using KH1's own no-damage timer.
> - **t544**: on KH1's states, when a guard stops one of our KH2 hits, it now goes through KH1's own clash, so Sora actually recoils. Before, nothing played.
>
> **Two findings:**
> - In KH2's own data, Explosion, Vicinity Break and Magnet Burst are pierce armor too. So at KH2 they never meet guards or attacks, and t544 only matters at Recoil KH1.
> - Question 4 is now the Recoil row: at KH1, KH1's 8 frames after a recoil; at KH2, none.
>
> **KH2's special guard** (you asked): KH2 has two kinds of guard.
> - A **plain guard** stops normal attacks. Pierce armor such as Guard Break goes straight through it. Almost every enemy guard in KH2 is this kind, which is why Guard Break breaks guards.
> - A **special guard** also stops pierce armor. In KH2's files (47 rows), nearly all of them belong to Sora and his allies: Aerial Recovery's invulnerable body guard, Reflect's barrier, Anti Form, Riku, King Mickey, Hercules and the Hollow Bastion fighters. Only three enemies have one: Xemnas's shield and two other Heartless/Nobody models I couldn't name (M_EX420, M_EX600_LC).
> - So it's the "nothing gets through this" block. For (c) I added an enemy list. It's empty for now, so at KH2 Guard Break meets no KH1 guard until we name enemies for it.
>
> **Checked:**
> - KH1 and KH2 Steam decomps: the recoil functions, KH2's 50 push and hop, and the 80 frames.
> - The KH1 exe: KH1's pairing table.
> - Your files: the request slots 72 / 73 / 74, the lengths of 41 and 43, every ATKP kind of Sora's moves, and the special guard owners.
> - Not checked in game. Two assumptions: KH1 runs one reaction per hit pass, and KH2's 80 counts 60 Hz frames.
>
> **Two places it still isn't KH2:**
> - A "refact self 2" move on KH1's states goes straight through an enemy's guard and damages the enemy behind it. In KH2 that enemy would just be immune to that swing. Fixing it needs a hook on KH1's clash.
> - An enemy attack of priority 0x62 or more is never stopped by a swing. That's KH1's rule; KH2 stops it.
>
> Note: this round the combo-row thread also added a **Guard Repel** row (Movement), which is what Sora's own KH2 guard does when it blocks. The Recoil row here is about his attacks meeting an enemy's attack or guard. They're separate switches.
>
> Please test, with Recoil at KH1 and then at KH2:
> - Swing into an enemy's attack.
> - Use Horizontal Slash and Guard Break into guards (Large Body's front) and into enemy attacks.
> - Guard an attack, then take a hit within about a second.
> - Use Explosion into a guard.
>
> A log and a Select + Triangle trace would help.
>
> Questions:
> 1. At Recoil KH1, Guard Break now uses KH1's priority, so guards repel it, as "KH1's rules exactly" says. Should it (a) stay like that, or (b) keep passing guards at KH1 as before?
> 2. For "refact self 2" moves meeting a guard on KH1's states: (a) leave it (they hit through the guard), or (b) I add the clash hook so the guarded enemy is immune, as in KH2?
> 3. Are there KH1 enemies whose guards should stop Guard Break (the special guard list)? (a) none for now, (b) these ones: ...

^m2
1. guard break should function like how it does in kh2, it should pierce any normal enemies and when we're ready to figure out how special guard works for in the game then we can add to your list. I also noticed that guard break and other kh moves still use the kh2s recoil mechanics when I set recoil to kh1, I've recorded a trace and log showing it off, I want the recoil setting in the config to change the system fundamentally so all kh2 moves behave just how a kh1 move would, if I change it to kh2 then all the kh1 and kh2 moves behave how kh2s recoil system works. (same with parry, everything should work how kh1 handles when you hit an enemy with guard, your recoil animation, parry, etc...) we need to also combine guard repel and recoil into the same config
2. add it and include in the kh2 config for recoil
3. (a) we'll get back to this list later

go ahead


