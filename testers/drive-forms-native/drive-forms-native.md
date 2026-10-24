# Drive Forms native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. This one covers Drive Forms, the Drive gauge, form levels, the missing forms and Drive orbs.
>
> **What we have now on testing.** Valor and Wisdom are in the Drive list, and Final only comes out with the secret pad code. There's no Limit, Master or Anti. A form runs entirely on KH2's controller: none of base Sora's native work (KH1 states, KH2 motions as KH1 slots, KH2 moves as KH1 attack rows, KH2 physics in KH1's fields) applies inside a form. A form costs 3 MP, drains yellow MP, and reverts at 0 MP. The level is fixed at 1 and the growth ability is always MAX. There's no Drive gauge and there are no orbs.
>
> **What KH2 does** (Steam KH2 decomp):
> - The gauge lives in Sora's status record: points `+0x1B0` (0..100), bars `+0x1B1`, max bars `+0x1B2`, and a mode byte `+0x1AF` (1 normal, 2 form, 3 summon).
> - `0x1403C0AD0` adds points and carries them into bars. Hits add through `0x1403A9CF0`.
> - Inside a form, that same add goes to the form gauge instead (`+0x1B4` / `+0x1B8`, in frames, changed by `0x1403C0B50`). In Anti a gain becomes a loss.
> - Entering a form (`0x1403C17C0`) refills HP and MP and sets the form gauge to (level + 3) bars of 600 frames. The level comes from the save's form records (`+0xE04 + (form − 1) × 0x38`, level at `+2`).
> - The form drains one frame per frame, half out of battle, times Form Boost (player step `0x1403A92C0`). It reverts at 0 (`0x140404A40`).
> - Drive orbs are prize types 7 and 8, worth 1 and 3 (the pickup row table at `0x1405C7420`, read from the exe).
>
> **How KH1 matches it** (Steam KH1 decomp):
> - KH1 has no gauge.
> - **Hits:** a hit lands in `0x1402BF330`, which already charges MP there (`0x1402A4AE0`). The Drive add belongs in the same spot.
> - **Orbs:** KH1's prizes are made by `0x1402AB2A0` and dropped by enemies through `0x1402ABC40`. They're picked up by `0x1402AC080`, whose switch only rewards types 0..6. Their rows are 7 rows at `0x140528BE0`. So Drive orbs fit as two new prize types run by KH1's own spawner and pickup.
> - **HUD:** the gauge can be drawn inside KH1's own HP/MP gauge task (`0x14026DB00`) with KH1's sprite calls.
>
> **Making the forms native.** The idea is to do for a form exactly what base Sora already has. On the change, Sora's actor keeps its object, and we point these at the form's own copies:
> - his slot table and request table: the form's KH2 motions as KH1 slots;
> - his attack row table: the form's combo rows as KH1 rows;
> - the physics numbers: the form's run speed, jump and gravity.
>
> On revert they all go back. The Drive motion plays as a KH1 slot, with KH1's intangible flag for its invulnerability. That gets rid of the second simulation inside forms, which is the main cost right now. Some form moves have no KH1 state yet and would be built on KH1's pieces:
> - Valor's second blade: a second attack object per row.
> - Wisdom's shots: KH1's free-standing attack objects, the kind spells and Wakka's ball use.
> - Final's glide: KH1's glide state with Final's numbers, and its blades' hits tied to the glide motion.
>
> **Plan, one system per version:**
> 1. The Drive gauge data, filled on hits by KH2's rules. Forms cost bars instead of MP.
> 2. The gauge on KH1's HUD.
> 3. The form gauge, form levels and form EXP. The growth ability follows the level (Lv1/3/5/7 give LV1/2/3/MAX).
> 4. Drive orbs as KH1 prizes.
> 5. Valor native, then Wisdom, then Final, then the missing Master, Limit and Anti (with KH2's Anti roll).
>
> **Not found yet:** on Steam, KH2's Drive cost check, the Anti counter, and where form EXP is added. On the KH1 side, free save bytes for the gauge and levels, and whether KH1's attack object can follow a second weapon bone.
>
> The notes are in `docs/kh1-drive-forms-native.md`.
>
> Questions:
> 1. KH2 raises the maximum Drive bars through story bonuses, and KH1 has none of those. Should it grow (a) at KH1 boss wins, (b) with Sora's level, or (c) by one per form unlocked?
> 2. How should forms unlock? (a) at KH1 story points, which you'd pick, (b) all from the start, or (c) as they do now on testing (Valor and Wisdom open, Final by the code)?
> 3. Anti Form: should it come from KH2's random roll in battles (a) as in KH2, (b) only by the code, or (c) not at all for now?

^m1


