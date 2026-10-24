# KH2 summons on KH1's summons (native research)

Research only (t480 round, thread `testers/kh2-summons`). Nothing here is built yet. All addresses are
**Steam** (from the Steam decomps) unless marked; where an older note gave an Epic address it is
translated through `crates/kh1-hook/src/win/steam_map.in` and said so.

## What the mod does with summons today (testing branch)

- Summons are KH1's, unmodded (`win/shared.rs`, `win/combat.rs` "KH1-only move"). **Checked (code)**
- The Summon list (menu 3, Epic `0x1405296A8` = Steam `0x1405289A8`) refuses a confirm while a KH2
  move can't be cut, like Magic (`combat.rs`, `from_summons`). **Checked (code)**
- The Drive page reuses KH1's Summon row and list for the forms (`win/drive.rs`; the summon-out flag
  Epic `0x14298D1A0` = Steam `0x14298C7A0`). **Checked (code)**
- KH1's summon movement states (38 Simba's charge, 40 the call) are KH1's (`takeover.rs`). **Checked (code)**
- The testing branch has **no Drive gauge**: forms cost MP (`win/form.rs`, `COST_MP`). **Checked (code)**

## KH1's summon system (Steam KH1 decomp)

- **Menu.** Command 6 "Summon" opens menu 3; entries are commands `0x2F + summon`, command type 6
  with the summon index in bits 8.. (`docs/kh1-drive.md`). **Checked (code / earlier notes)**
- **Can summon** `0x140288F30 (summon)`: false when any of: a world flag (`0x142D53A38 & 4`), Sora's
  movement state record (`0x142DB3840 + state*0x10`) lacks bit 0x80, `0x142867364` bit 0x10000, his
  current MP (stats `+0x44`) is under the summon's cost, or not in battle (`0x142867364` bit 0). Each
  summon only once per battle (mask `0x14298C7AC`; not enforced in world 3 room 0x14). It also walks
  the three party slots (`0x142D37280..`): every partner must be there and able (`0x1402B0030`).
  **Checked (decomp)** (the room exception's meaning: **Assumption**, probably a tutorial room).
- **Start** `0x140289210 (actor, summon)` (from the command step `0x1402A3DB0`, type 6):
  - battle flags `0x142867364 |= 0x102000` (0x2000 = a summon is out);
  - `0x14298C798` = the summon's 8-byte record at `[0x142D22D30] + 0x98 + summon*8` (btltbl's summon
    table): `+0` MP cost, `+1` gauge drain per frame, `+2` drain per special action;
  - `0x14298C7A0` = summon index | 8 (out); `0x14298C7AC |= 1 << summon` (used this battle);
  - **summon time** `0x14298C7A4` = max `0x14298C7A8` = (stats `+0x48` + ability bits
    `0x142D5EC10 >> 10 & 7`) × 3000. **Checked (decomp)**; that `+0x48` is max MP: **Assumption**
    (`+0x44` is the current MP in the same check); what the ability bits 10..12 are: not found.
  - MP paid: `0x1402A4A00 (actor, -cost)`;
  - three party slots' stream records cleared (`0x1402908D0 (1/2/3)`, records `0x142D371D0 + i*0x28`);
  - the summon's file loaded (`0x1402858E0(0x34)`, async; stage `0x14298C8F4` 1 → 2 → 4; Genie /
    Simba load `genie.dpx` / `simba.dpx` too, so index 2 = Genie, 5 = Simba) and a sequence task at
    priority 52000 (`0x14028A410`, its steps from `0x140528230`). **Checked (decomp)**
- **Arrival** `0x140289750` (a step of that task): partners other than Sora set aside
  (`0x140292160`), the summon actor found as the actor whose kind byte `+6` is 7, Sora's motion set,
  KH1's event camera callback (`[0x1406AAD88]`). **Checked (decomp)**
- **Per summon** each summon is a hand-written state machine in `0x140395320..0x14039C540` (e.g.
  `0x140395360`): every frame it takes `record[+1] × frame scale (0x14233FBDC)` off the time,
  `record[+2]` more during its special, ends when `0x1402894A0` says the time is out
  (`<= 0` or not out), and puts its gauge on the HUD with `0x140268D00 (time, ..., max, ...)` (HUD
  slot 3, fields `+0xB60..+0xB78`). **Checked (decomp)**
- **Leaving** `0x140289060` / `0x140289620`: partners back (`0x140292160`, `0x1402A0F40`, flag
  0x01000000 cleared, `0x140297F30`), `0x142867364 &= ~0x2000`, `0x14298C7A0 = 0`. Cleanup on room /
  battle reset `0x140289410` (clears 8, the used mask, flags `0x106000`). **Checked (decomp)**
- **What KH1 reads elsewhere** (don't break): the menu fill greys / swaps Summon for `0x4F` while
  `0x14298C7A0 & 8`; the item step calls summon 1's handler on an item use (`0x1402A4FF0` area →
  `0x140395AF0(5, item)`); Sora's death check skips while summon 3 is out with `0x2000` (so summon 3
  saves him: **Assumption** it is Tinker Bell); the summon scripts read the time through
  `0x1402894A0`. Journal / tally counting of summons: not looked for.

## KH2's summon system (Steam KH2 decomp, KH2 files)

- **Commands** (03system `cmd`, execute type 0x48): 0x58, 0x59, 0x132, 0x1BF, from the charm items
  0x19, 0x9F, 0xA0, 0x17F (item type 20). Every one costs **3 Drive bars** (cmd byte 0x1C).
  **Checked (files)**. Names by OpenKH's lists: 0x58 Chicken Little, 0x59 Genie, 0x132 Stitch,
  0x1BF Peter Pan (**Assumption**, not read from msg; matches the Genie / Stitch code below).
- **Can summon** `0x140401D60` (command availability), case 0x48: as Drive (case 4) plus an area
  flag (`0x1403A3AF0`: the map's record bit 14 forbids summons): not in a form or changing, Sora with
  his party flag, not in certain states (`+0xDE0` 7 / 0xC), world flag `0x142A11400` bit 18 clear,
  and Drive bars (`[+0x5C0] + 0x1B1`) at least the cost. `0x1404005E0`: **both partners must be
  present and able** (`0x1403C3270(i)`, i < 2), unless `0x1403A3BA0` allows it. No once-per-battle
  rule seen. **Checked (decomp)**
- **Execute** `0x140405248`, type 0x48: Sora's action 0x8A (blend 8), system effect 0x13, the cost
  paid from the Drive bars (`0x1403C2780`, the same payment as a form), then `0x1403DE460 (sora, cmd)`.
  **Checked (decomp)**
- **Start** `0x1403DE460`: summon level from the command (`0x1403E6BE0`); Genie (0x59) picks a random
  Drive form Sora has (1..5, not 3) to copy; Stitch (0x132) a fixed variant 1; a task
  `0x1403DE610` runs the arrival. **Checked (decomp)**
- **Arrival** `0x1403DE610`: the full "appear" event the first time (or always) by the Config option
  **Summon Effects** (save config `+0x41A4` bits 9..10, `0x1402E4FA0`: 1 always, 2 first time per
  summon, bits `0x142A24ED8`); otherwise "appear_skip" (the summon placed 200..300 from Sora). Then
  `0x1403C2740 (stats, 3, level)`: the **form gauge becomes the summon gauge** (kind byte `+0x1AF` = 3)
  of `(summon level + 3) × param[+0x68]` (the same bar length as forms: (level + 3) × 600 frames,
  `docs/drive-model.md`). The summon object (vtable `0x14074CB48`, `SUMMON@YS`, a party "friend"
  class with its own script) joins the party slots `0x142A24EE8`. **Checked (decomp)**
- **Gauge**: damage to the summon takes `damage × param[+0x68] / its max HP` off the gauge
  (`0x1403DEA20` → `0x1403C0B50`). The time drain itself is the form gauge's (1 a frame in battle):
  **Assumption** (`0x1403C0B50` handles kind 3; the per-frame caller not traced).
- **Summon levels** 00battle `fmlv` form 0: levels 1..7, EXP 6 / 16 / 25 / 42 / 63 / 98.
  **Checked (files)**. How summon EXP is earned: not found yet.
- **Not traced yet**: the summon's Limit (Sora's Triangle command with the summon, which uses MP in
  KH2: **Assumption** from play), how the summon ends (gauge out, the limit's end, Sora KO), the
  partners' return, Drive gauge refill rules during a summon, summon EXP.

## What can run on KH1's summons, and what can't

KH1's summons are different characters (Simba, Genie, Bambi, Dumbo, Tinker Bell, Mushu) with their own
hand-written AI; KH2's are other characters with BDX scripts and their own limits. So KH2's
**rules** can run on KH1's summon system; KH2's **summons themselves** (Chicken Little, Stitch, Peter
Pan, KH2's Genie and their limits) would be new actors, not native.

| KH2 rule | On KH1's summon | Native? |
|---|---|---|
| Cost: 3 Drive bars | KH1 pays MP at `0x140289210`; we'd pay bars instead | Needs a Drive gauge (the testing branch has none) |
| Both partners able | KH1 already requires it (`0x140288F30`) | Already native |
| No once-per-battle | KH1's mask `0x14298C7AC` | Clear the bit / skip the check |
| Not in a form / area flag | Our form state; KH1 world flags | Native check in `0x140288F30` |
| Gauge (level + 3) × 600 | KH1's time `0x14298C7A4` / `+A8` and its HUD gauge | Write KH2's time into KH1's fields; drain as KH2 (KH1's drain bytes overridden) |
| Damage drains the gauge | KH1 summons take no gauge damage: **Assumption** | Hook the summon actor's damage |
| Summon levels / EXP | none in KH1 | New save bytes (EXP rule still to find) |
| Summon Effects option | KH1 always plays its arrival | A Config row; KH1's arrival sequence would need a skip path |
| Summon Limit (Triangle) | KH1's summons have specials on their own AI, no Sora command | Would need KH1's special triggered from a Sora command: design choice |

**Performance**: all of it rides KH1's existing summon task and fields: one hook on the start
(`0x140289210`), one on the can-summon check (`0x140288F30`), a write to the time each frame (or a
patch of the drain bytes); no parallel simulation.
