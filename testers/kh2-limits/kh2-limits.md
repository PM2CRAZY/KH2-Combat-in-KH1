# KH2 Limits

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. This one is KH2's Limits.
>
> **What we have now.** Nothing of KH2's. The testing branch has no Limit Form and no KH2 limit code. KH1's own five limits (Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit) work as in vanilla KH1 for base Sora, and Valor Form refuses them.
>
> **What KH2 does** (KH2 decomp and its data files):
> - Every limit is a command with "execute 81" and an MP cost of 255 (all MP). 00battle's `limt` table gives each one its file (`limit/<lang>/donald.bar` and so on) and who takes part: Sora, Donald, Goofy or a guest.
> - Starting one (`0x1403D8B40`) makes a limit object that holds Sora, the partner(s) and a time gauge. It empties his MP into MP Charge and runs the limit file's script.
> - A command picked while it runs is sent to that script as a follow-up. That's how the Triangle follow-ups work.
>
> **How KH1 does its own** (KH1 decomp):
> - The Triangle offer is `0x14028D0F0`. KH1's command dispatch sends the limit commands to `0x14028D3D0`, which starts limit k with `0x14028D800`.
> - The start sets the "limit running" bit (16 of `0x142867364`) and swaps Sora's motion table for the one in `xl_limit.dat`.
> - Each limit is its own KH1 movement state (Sonic Blade 0x20, Ragnarok 0x25, Strike Raid 0x27, Trinity 0x29). Its hits use Sora's normal attack rows.
> - That's the same path our native moves already use, so KH2's limits can go in as more of KH1's own limits.
>
> **Which KH2 limits fit KH1's party:**
> - Donald: Comet, Duck Flare.
> - Goofy: Whirli-Goof, Knocksmash.
> - Both: Trinity Limit.
> - Guests: Aladdin (Session), Jack (Applause Applause), Beast (Howling Moon).
> - Auron, Mulan, Sparrow, Simba, Tron and Riku have no KH1 partner.
> - Limit Form's four would need Limit Form on this branch first.
> - The KH2 names for each file are from memory of the game, not read from its text.
>
> **Plan, one system per version:**
> 1. KH2 limit commands go through KH1's limit start: its "limit running" bit, a motion table holding KH2's baked limit motions, and a KH1 movement state of our own.
> 2. The limit file's script runs from that state, answered on KH1's actors.
> 3. The follow-ups go in KH1's own Triangle box.
> 4. The hits go through KH1's attack rows with KH2's ATKP numbers.
> 5. Donald and Goofy play KH2's partner motions on their KH1 models. This is the biggest piece: nothing retargets the partners yet.
> 6. MP and the time gauge.
>
> **What I couldn't find yet:**
> - How the partners are moved in either game.
> - KH2's limit timer.
> - What two of KH1's limit flags do (0x200000, 0x80), and whether KH2's +0x124 bit 0x10000 is the invulnerability.
> - What's inside the limit `.bar` files (I haven't opened them yet).
>
> The full notes are in `docs/kh1-kh2-limits.md`.
>
> Questions:
> 1. Which limits first? (a) Donald, Goofy and Trinity, (b) those plus the guests (Aladdin, Jack, Beast), (c) only Limit Form's, once Limit Form exists.
> 2. MP: KH2 empties your MP into MP Charge, which isn't in yet (it's its own thread). (a) Wait and do it KH2's way with MP Charge, (b) use KH1's limit costs until MP Charge is in, (c) empty the MP but refill it the KH1 way.
> 3. KH1's own five limits with KH2 limits on: (a) replace them in the Triangle box with KH2's, (b) keep both, (c) a Config row "Limits: KH1 | KH2".
> 4. Where should KH2's limits be picked? (a) in the command menu as KH2 does (I still have to trace which row KH2 uses), (b) in KH1's Triangle box, (c) both.

^m1


