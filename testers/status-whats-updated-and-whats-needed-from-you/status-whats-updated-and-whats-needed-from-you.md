# Status: what's updated and what's needed from you

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: You asked for this in Quick Run native(../quick-run-native/q](#^m1)
- [2. Assistant: Later in the same round (no new build):](#^m2)
- [3. Assistant: Latest build: t515. Your startup crash: t514 fixes the likel](#^m3)
- [4. Assistant: Will do: from now on every round ends with a message here in](#^m4)
- [5. Assistant: Latest build: t631 (2026-10-25; the build bot is making it n](#^m5)

---


> [!tip] Assistant
> You asked for this in [Quick Run native](../quick-run-native/quick-run-native.md). I'll update this thread every round, with a new message at the bottom: what changed, which thread it's in, and what each thread needs from you. You don't need to reply here, but you can if you like.
>
> **Latest build:** t513 (2026-10-24).
>
> **Updated this round:**
> - [Missing sleeves animation](../missing-sleeves-animation/missing-sleeves-animation.md): t499 (thighs and shins twist as KH2's legs do), t500 (coat measuring tool, no behaviour change).
> - [Quick Run native](../quick-run-native/quick-run-native.md): t501 to t505. Quick Run is its own KH1 state, LV1 has 0 invulnerable frames, and there's a start effect at LV2+.
> - [Sora's hit reactions native](../soras-hit-reactions-native/soras-hit-reactions-native.md): t506 to t508. New rows: Hit Reactions, Knock-back Numbers, Medium Hits.
> - [KH2 sounds through KH1's sound system](../kh2-sounds-through-kh1s-sound-system/kh2-sounds-through-kh1s-sound-system.md): t509 to t513. KH2's effects and quotes go through KH1's sound engine.
> - [KH2 recoil and parry system](../kh2-recoil-and-parry-system/kh2-recoil-and-parry-system.md): new thread, research only.
> - This status thread: new.
>
> **Earlier today, still to test:**
> - [Aerial Dodge native](../aerial-dodge-native/aerial-dodge-native.md): t492 to t496.
> - [KH2 Camera fixes](../kh2-camera-fixes/kh2-camera-fixes.md): t497 (lock-on side kick), t498 (L2 Camera Hold row).
>
> **What each thread needs from you:**
>
> | Thread | Needs |
> |---|---|
> | Aerial Dodge native | test t492 to t496, plus one question (body-guard parry on other moves) |
> | KH2 Camera fixes | test t497 (a left/right trace), t498; 2 questions (L2 d-pad use, the row's default) |
> | KH2's combo row selection | 1 question (the Guard Repel row: KH2 already matches) |
> | Dodge Roll, Guard and Counterguard native | "go ahead" for the native roll |
> | Missing sleeves animation | test t499; 1 question (the coat) |
> | Quick Run native | test t501 to t505; 2 questions |
> | Sora's hit reactions native | test t506 to t508; 3 questions |
> | KH2 sounds through KH1's sound system | test t509 to t513 and send the log; 3 questions |
> | KH2 recoil and parry system | answers to 4 questions, a log and trace, "go ahead" |
> | Drive Forms native | nothing for now (on hold until forms go native, as you said) |
> | Every other thread | waiting for your reply to my last message there |

^m1


> [!tip] Assistant
> Later in the same round (no new build):
> - [KH2 magic and Reflect native](../kh2-magic-and-reflect-native/kh2-magic-and-reflect-native.md): research on air casts for both physics settings, and on Cure KH1. Needs 2 answers and a "go ahead".
> - [KH2 Glide in KH1's glide state](../kh2-glide-in-kh1s-glide-state/kh2-glide-in-kh1s-glide-state.md): my take on your three answers. Needs a "go ahead".

^m2


> [!tip] Assistant
> **Latest build: t515.** Your startup crash: t514 fixes the likely cause (KH1's voice-bank registration got a missing argument), and t515 makes `kh2_sound_engine = mod` a full switch-back. Details in [KH2 sounds through KH1's sound system](../kh2-sounds-through-kh1s-sound-system/kh2-sounds-through-kh1s-sound-system.md). Needed from you: start the game, and if it still crashes, push `kh1-kh2.log` into that thread's folder.

^m3


make sure you update this everytime you do an update, the exact same way you replied the first time on anything new.

go ahead


> [!tip] Assistant
> Will do: from now on every round ends with a message here in this same layout (latest build, what changed and where, what each thread needs).
>
> **Latest build:** t567 (2026-10-25).
>
> **Updated this round:**
> - [Aerial Dodge native](../aerial-dodge-native/aerial-dodge-native.md): t545 (at Physics KH1 the dodge is held as KH1's own jump, so it's no longer cut 6 frames in). Levels checked, unchanged.
> - [Body guards on KH2 moves](../body-guards-on-kh2-moves/body-guards-on-kh2-moves.md): new thread (your 7 (a) from Aerial Dodge). t546: Aerial Recovery, Explosion and Magnet Burst's body guards parry like the dodge's.
> - [Quick Run native](../quick-run-native/quick-run-native.md): t547 (out of KH1's roll at KH2's frame 10).
> - [KH2 Glide in KH1's glide state](../kh2-glide-in-kh1s-glide-state/kh2-glide-in-kh1s-glide-state.md): t548 (left bank), t549 (speed kept when you let go).
> - [Sora's hit reactions native](../soras-hit-reactions-native/soras-hit-reactions-native.md): t550 (KH2's hurt lines), t551 (KH2's air flinch turn).
> - [Missing sleeves animation](../missing-sleeves-animation/missing-sleeves-animation.md): t552 (the coat moves with KH2's coat bones in every baked animation).
> - [KH2 recoil and parry system](../kh2-recoil-and-parry-system/kh2-recoil-and-parry-system.md): t553 to t556 (Guard Repel merged into Recoil, Recoil KH1 behaves as KH1 for KH2 moves, Guard Break pierces, refact self 2 at KH2).
> - [KH2's combo row selection](../kh2s-combo-row-selection/kh2s-combo-row-selection.md): t557, t558 (reimplemented: KH1's own row picker picks the KH2 ability moves at Combo KH1).
> - [KH2 hit effects on enemies](../kh2-hit-effects-on-enemies/kh2-hit-effects-on-enemies.md): t559 (KH2 Cure centred), t560 (KH2 swings' Kingdom Key hit spark).
> - [KH2 effects in KH1's own effect system](../kh2-effects-in-kh1s-own-effect-system/kh2-effects-in-kh1s-own-effect-system.md): t561 to t564 (Fire's size, a Config row per spell plus Animations, Gravity / Stop / Aero on KH2 cast slots, the Air Physics row).
> - [KH2 sounds through KH1's sound system](../kh2-sounds-through-kh1s-sound-system/kh2-sounds-through-kh1s-sound-system.md): t565 to t567 (the old player removed, one copy of each bank, battle quotes play).
> - [KH2 attack data and damage on KH1's swings](../kh2-attack-data-and-damage-on-kh1s-swings/kh2-attack-data-and-damage-on-kh1s-swings.md): research reply, no code (no go-ahead yet).
> - [KH2 spell hit sounds](../kh2-spell-hit-sounds/kh2-spell-hit-sounds.md): new thread (your 2 (b) from hit effects), plan only.
>
> **What each thread needs from you:**
>
> | Thread | Needs |
> |---|---|
> | Aerial Dodge native | test t545; 1 question (keep KH1's release hop at Physics KH1) |
> | Body guards on KH2 moves | test t546; 2 questions (Flash Step / Vicinity Break, range id 44) |
> | Quick Run native | test t547; 1 question (the roll's speed before the cut) |
> | KH2 Glide in KH1's glide state | test t548, t549 (a trace helps); 1 question |
> | Sora's hit reactions native | test t550, t551; 3 questions |
> | Missing sleeves animation | test t552; 1 question for the clipping later |
> | KH2 recoil and parry system | test t553 to t556 (Recoil both ways); 3 questions |
> | KH2's combo row selection | test t557, t558 at Combo KH1; 3 questions |
> | KH2 hit effects on enemies | test t559, t560; 4 questions (and the link didn't come through: describe it in words) |
> | KH2 effects in KH1's own effect system | test t561 to t564 (the new Magic lists); 4 questions |
> | KH2 sounds through KH1's sound system | test t565 to t567, watch RAM; 2 questions |
> | KH2 attack data and damage on KH1's swings | "go ahead" for criticals off for everyone and the KH2 Damage row |
> | KH2 spell hit sounds | 1 question and "go ahead" |
> | Every other thread | waiting for your reply to my last message there |

^m4


give me an update

> [!tip] Assistant
> **Latest build:** t631 (2026-10-25; the build bot is making it now).
>
> **Updated since the last status:**
> - [Air physics for KH2 magic, Aerial Finish and Magnet Burst](../air-physics-for-kh2-magic-aerial-finish-and-magnet-burst/air-physics-for-kh2-magic-aerial-finish-and-magnet-burst.md): t621 (Aerial Finish and Magnet Burst back on the main Physics row). The fall after an air cast matches the Steam exe frame for frame, so it isn't changed; a KH2 recording would help.
> - [KH2's combo row selection](../kh2s-combo-row-selection/kh2s-combo-row-selection.md): t622, t623 (KH2's pick made inside KH1's picker, one copy of KH2's rules), t629 (Zantetsuken follows the Tweak), t630 (no more KH2-controller hand-offs in the combo), t631 (every KH2 combo move on its own KH1 row).
> - [KH2 magic and Reflect native](../kh2-magic-and-reflect-native/kh2-magic-and-reflect-native.md): t624 (Finishing Plus after a finisher cast).
> - [Baking the rest of the base animations](../baking-the-rest-of-the-base-animations/baking-the-rest-of-the-base-animations.md): t625 (item use), t626 (the fall and KH2's switch into it), t627 (jump and High Jump), all as KH1 slots.
> - [Missing sleeves animation](../missing-sleeves-animation/missing-sleeves-animation.md): t628 (the hair switches out again).
>
> **What each thread needs from you:**
>
> | Thread | Needs |
> |---|---|
> | Air physics for KH2 magic, Aerial Finish and Magnet Burst | test t621; what KH2 shows after the hover that ours doesn't (3 questions), a KH2 clip helps |
> | KH2's combo row selection | test t629 to t631 (a trace helps); 2 questions |
> | KH2 magic and Reflect native | test t624 with Finishing Plus; 1 question |
> | Baking the rest of the base animations | test t625 to t627; 1 question (what answer 1 meant, for idle and landing) |
> | Missing sleeves animation | check t628 |
> | What is still missing | the updated list there |
> | Every other thread | waiting for your reply to my last message there |

^m5

