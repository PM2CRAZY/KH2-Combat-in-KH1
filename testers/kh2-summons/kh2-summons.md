# KH2 summons

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. It's research only so far; nothing is built.
>
> **What we have now.** Summons are KH1's, as in the unmodded game. The only mod changes are that the Summon list waits for KH2's move windows the way Magic does, and that the Drive page reuses the Summon row and list. The testing branch also has no Drive gauge yet (forms cost MP), and that matters below.
>
> **What KH2 does** (KH2 decomp and files):
> - The four summons (Chicken Little, Genie, Stitch, Peter Pan) each cost **3 Drive bars**. That's from 03system's cmd table.
> - You can summon with the same checks as Drive (not in a form, enough bars), plus two more: the area mustn't forbid summons, and **both partners must be there and able**. I saw no once-per-battle rule.
> - Once the summon is out, the form gauge becomes the summon gauge: **(summon level + 3) × 600 frames**, the same formula as forms. Damage the summon takes also drains it.
> - Summon level runs 1 to 7, with EXP 6 / 16 / 25 / 42 / 63 / 98 from the fmlv table.
> - Genie copies a random Drive form Sora has.
> - The Summon Effects option picks the full arrival (always, or the first time only) or a skipped one.
>
> **How KH1 does it** (KH1 decomp):
> - `0x140288F30` checks you can summon: battle mode, MP at least the cost, both partners there and able, and each summon **only once per battle**.
> - `0x140289210` starts the summon. It pays the MP cost, then sets the summon time to (`+0x48` + some ability bits) × 3000. I think `+0x48` is max MP, but that's an assumption. It also marks the summon out (`0x14298C7A0`) and starts a sequence task that sets the party aside.
> - Each summon is its own hard-coded AI. Every frame it drains the time by its table byte, plus a second byte during its special. It shows the time on KH1's own HUD gauge and leaves when the time runs out.
>
> **What fits.** KH1's summons are different characters, so KH2's *rules* can run natively on KH1's summon system. KH2's own summons and their Limits would be new actors, not native. A plan, one system per version:
> 1. Summon gauge: KH2's (level + 3) × 600 written into KH1's time fields, drained at KH2's rate, on KH1's HUD gauge.
> 2. Summon rules: no once-per-battle limit, not in a form, partners required (KH1 already does this).
> 3. Cost: 3 Drive bars. This needs a Drive gauge, which the testing branch doesn't have.
> 4. Damage to the summon drains the gauge.
> 5. Summon levels and EXP: new save data. I still have to find KH2's EXP rule.
> 6. A Summon Effects Config row (skip KH1's arrival).
>
> **Couldn't find yet:** how KH2 summons earn EXP; how a KH2 summon ends and gives the partners back; the summon Limit (Sora's Triangle command with the summon, MP in KH2, but I haven't checked that in the decomp); and what KH1's ability bits 10..12 in the time formula are. The notes are in `docs/kh1-kh2-summons.md`.
>
> Questions:
> - Cost while there's no Drive gauge: (a) wait for a native Drive gauge and use 3 bars; (b) keep KH1's MP cost until then; (c) use KH2's 3 as 3 MP bars for now.
> - KH2's summon Limit has no KH1 equivalent. Should Triangle during a summon (a) trigger the KH1 summon's own special on command, (b) stay out, or (c) be researched first as its own thread?
> - Should Genie copy a Drive form (KH2's Genie)? KH1's Genie is a different character: (a) yes, Genie only; (b) no.

^m1


