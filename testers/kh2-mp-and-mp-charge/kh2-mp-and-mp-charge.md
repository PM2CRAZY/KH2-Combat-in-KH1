# KH2 MP and MP Charge

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: Started from What is still missing(../what-is-still-missing-](#^m1)

---


> [!tip] Assistant
> Started from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) at your request ("Not in at all: KH2's MP and MP Charge"). This is research only so far, nothing is built.
>
> **What we have now:** everything runs on KH1's own MP (whole points, with the yellow part under them). Every KH2 spell and Reflect costs 1 KH1 point, Drive Forms cost 3 and drain the yellow part, and there's no MP Charge. Hits Sora lands still fill MP the KH1 way, KH2 swings included.
>
> **What KH2 does (KH2 decomp, 03system):**
> - If a cost would take MP below 0, MP Charge starts instead and MP goes to 0. So Cure (cost 255, "all") always empties MP and starts the charge.
> - The charge length comes from prty's "mp_drive" value, which is 3000 for every moveset. That's 50 seconds at the base rate. When it runs out, MP refills to full.
> - While it charges, any MP Sora gains (orbs, MP Rage, items) doesn't add MP. Each point takes 12 units (0.2 s) off the time left instead.
> - Dealing hits never gives MP. MP Rage gives MP when he's hurt: damage / max HP × a rate.
> - MP Haste, Hastera and Hastega speed up the charge, but I haven't found where their numbers are set yet.
>
> **How KH1 does it (KH1 decomp, btltbl):**
> - The next MP point needs (MP + 1) × 30 yellow units, and the player step turns a full yellow part into a point every frame.
> - Landing a hit adds units from the attack data (+0x2C), scaled by level difference. KH1's MP Haste adds 150% to that.
> - KH1's MP Rage adds damage × copies when he's hurt, and MP orbs add 2 or 15 units.
> - Costs of 100 or more are whole points; costs under 100 are yellow units. That's how the form drain works.
> - What reads KH1's MP: spells, summons, specials, party casts, items, the HUD bar, save points, and one event script command that gives MP. I found no event that checks MP.
>
> **How this fits the magic thread:** there you picked (6b), KH2's costs scaled onto KH1's MP. That's the cost half of this. This thread covers what happens when the pool runs out and what refills it. Both will use one scale, built once.
>
> **Plan (KH2's rules on KH1's pool, one system per version):**
> 1. The cost scale, shared with the magic thread's (6b).
> 2. Running out starts the MP Charge: a payment that would go below 0 empties MP and starts a timer for Sora, and Cure takes all.
> 3. The charge ticks on KH1's own per-frame player step and refills max MP at the end.
> 4. Sora's hits stop filling MP, and orbs shorten the charge instead.
> 5. The charge is shown on KH1's own MP bar, using the yellow part as its fill.
> 6. KH1's MP Haste and MP Rage get KH2's meanings, plus Hastera and Hastega if you want them.
>
> Party members, summons and KH1's own spells would keep KH1's rules. It's a few small hooks and one timer, with no per-frame scans.
>
> **Couldn't find yet:** MP Haste's / Hastera's / Hastega's rates and MP Rage's rate per copy in KH2, KH2 Sora's max MP per level, and how KH1's HUD draws the yellow part.
>
> Notes: `docs/kh1-kh2-mp.md`.
>
> Questions:
> 1. Which spells go on KH2's MP rules? (a) only the KH2-style spells and Reflect, (b) KH1-style spells too, or (c) everything Sora pays for, summons and specials included?
> 2. Drive Forms cost and drain MP on this branch. If forms run him out of MP, should that (a) start the MP Charge like a spell, (b) just revert him with no charge, or (c) wait until forms move to their own Drive gauge?
> 3. The scale: (a) KH2's cost as a share of max MP (Fire is 12 of KH2's 100, so 12% of KH1's max MP, in yellow units so Fire, Blizzard and Thunder cost different amounts), or (b) whole KH1 points (rounded, so most spells cost 1 or 2)?
> 4. MP Charge's bar: (a) on KH1's own MP bar as the yellow part, or (b) KH2's pink "MP CHARGE" bar art drawn on KH1's HUD?

^m1


