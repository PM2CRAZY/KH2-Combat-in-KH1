# KH2 Limits in KH1

Research notes (t480 round) for the thread "KH2 Limits". Steam addresses throughout (image base
0x140000000). Where an address came from an older Epic note it was translated through the
neighbouring pairs of `crates/kh1-hook/src/win/steam_map.in` and then found in the Steam decomp at
that address; those are marked "(translated, found in the Steam decomp)".

## 1. What the testing branch has now

- **No KH2 limits at all.** No `limits.rs`, no Limit Form (`form::Kind` is Valor, Wisdom, Final),
  and testing's `kh2-magic` has no limit support (no trigger 22 / limit functions). **Checked (code)**
- KH1's own five limits (Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit) run as
  vanilla KH1 for base Sora (KH1's Triangle offer). In Valor Form `combat::command` / `combat::exec`
  refuse every Sora command KH1 runs as a move other than Attack, magic and items, which takes in
  the limit commands too. **Checked (code)**
- The main branch's AGENTS.md (s287 to s336) describes a Limit Form, a KH2 Limit menu running
  `limi` and KH1's limits in a form's Limit menu. None of it is on testing, and it was a parallel
  system (KH2's controller drove Sora); it is only a lead here.

## 2. How KH2 runs a limit (KH2 Steam decomp, 00battle / 03system)

- **Data.**
  - 03system `cmd`: limit commands have execute 81 (0x51), argument = their own id, MP cost byte
    255 ("all MP"), flags 0x110..0x150. Ids: 0x4A, 0x52, 0x8B, 0xFF, 0x11A, 0x152, 0x158, 0x15F,
    0x165, 0x178, 0x195, 0x19F, 0x1E4, 0x239, 0x26C. **Checked (files)** (local/re/03system.bin)
  - 00battle `limt` (31 records of 64 bytes): +0 id, +1..+3 the characters taking part (1 is Sora,
    2 Donald, 3 Goofy, others the guests), +4 the limit's file name (`limit/<lang>/<name>.bar`,
    format strings at 0x1405C9010 / 0x1405C9020), +0x28 the command id, +0x2A non-zero for the
    summon-like entries, +0x2C a world (0x0D for the Timeless River `_wi` versions). **Checked (files)**
  - Records by file: auron, beast (cmd 0x4A), mulan, goofy / goofy_wi (0xFF), goofy2 (0x152),
    donald (0x11A), donald2 (0x158), aladdin (0x15F), sparrow, simba, jack (0x195), tron, riku,
    trinity / trinity_wi / trinity_zz (0x1E4, characters 1 2 3), peterpan, stitch, chickenlittle,
    the four genie forms, and Limit Form's sonic_rave (0x2BA), last_alcanam (0x2BD), strike_raid
    (0x2C0), ragnarok (0x2AB). **Checked (files)**. The KH2 names (donald = Comet, donald2 = Duck
    Flare, goofy = Whirli-Goof, goofy2 = Knocksmash, beast = Howling Moon, aladdin = Session, jack =
    Applause Applause, trinity = Trinity Limit) are **Assumption** (from the game, not from the
    message table, which wasn't read).
- **Menu.** The command list builder (0x1403B1840 / 0x1403B19B0) swaps command 0x51 ("Limit") for
  the party's limit through 0x1403B2660(..., 2). **Checked (decomp)**; which menu row that is in
  KH2's layout was not traced.
- **Greyed or not.** 0x1403D88E0 (called for execute 0x51 from the command state check
  0x140401D60): no limit running (0x142A24CC0 null), the limit record found (0x1403E7800, which
  walks the `limt` table at 0x142AE5690 checking the world byte and that the party member is
  there), MP Charge not running (0x1403C1D10), MP compared with the cost. **Checked (decomp)**;
  the exact meaning of its return values 2 / 5 is **Assumption** (5 greyed).
- **Start.** 0x1403D8B40 (from the command executor 0x140405440):
  - makes a 0x128-byte limit object (0x1403D7FA0) that holds Sora, up to two partners from the
    record's character bytes (each flagged +0xDD4 = 1 and +0x124 |= 0x10000), a voice object and a
    `MISSION_GAUGE` (the limit's time gauge on the HUD); **Checked (decomp)**
  - pays the MP through 0x1403D3CF0(player, -cost): a cost above his MP starts MP Charge
    (0x1403C20A0) instead, so a 255 cost empties his MP into MP Charge; **Checked (decomp)** for
    the branch, **Assumption** that this is the "uses all MP" rule as seen in game;
  - calls 0x140400990 on each party member not already in it, and starts the limit (0x1403D9130).
  - +0x124 bit 0x10000 being the limit's invulnerability is **Assumption** (no reader found).
- **Follow-ups.** A limit command while one runs is not a new limit: 0x1403D8B40 sends trigger
  0x17 (23) with the command to the running limit's script (object +0x48). That is how the
  reaction-command follow-ups reach the script. **Checked (decomp)**
- **Per-limit actions.** KH2 has action classes `ACTION_LIMIT_FLY / _MULAN / _RIKU / _PETERPAN /
  _HOVER` (RTTI strings at 0x140796518..). Not read further.
- Not found: the timer length per limit, how partners' commands are cleared (0x140400990 not
  read), how the script's motions are picked from the limit `.bar`.

## 3. How KH1 runs its limits (KH1 Steam decomp)

- **The offer.** 0x14028D0F0 (from the context fill 0x1402948F0) puts one limit in the Triangle
  box (0x140293000(3, command), greyed with 0x140292FB0(3) when MP is short). It needs: battle
  flags 0x142867364 bit 16 clear (no limit running), 0x142D53A38 bit 1 clear, Sora (0x142D37280)
  in state 0, the limit data loaded (0x14299C220 == 5), a hittable target (0x142DB3518, `+0x374 &
  0x60` = 0x40). Pairs (ability, command) at 0x140528340: Sonic Blade 11 / 0x4B, Ars Arcanum 12 /
  0x57, Strike Raid 13 / 0x5E, Ragnarok 14 / 0x5A, Trinity Limit 15 / 0x52; tried in the order at
  0x14052834C; the equipped bits at 0x14299C240 (set by 0x14028D4A0 / 0x14028D550 from
  0x140284BE0). Range from the command record's +0xE; Sonic Blade only with the target under 160
  above him, Ragnarok only over 160; Trinity Limit needs both partners (0x142D37288, 0x142D37290)
  able and within 500. MP: `*0x142D22D30 + 0xF4 + 2k` through 0x1402A4C10. **Checked (decomp)**
  (all translated, found in the Steam decomp).
- **The command.** KH1's command dispatch 0x1402A3600 sends commands 0x4B 0x4D 0x4E 0x52 0x57..0x5B
  0x5E..0x60 to 0x14028D3D0 with the record at 0x142D53A40. Starts: 0x4B → k 0, 0x57 → 1, 0x5E → 2,
  0x5A → 3, 0x52 → 4. Follow-ups: 0x4D / 0x4E → 0x140397A50(7), 0x58 / 0x59 → (8), 0x5F / 0x60 →
  (9), 0x5B → (0xB). **Checked (decomp)**
- **The start** 0x14028D800(sora, k): sets bit 16 (clears bit 23), swaps Sora's motion table to
  the limit file's (0x14029E850: the old table kept at +0x1E0, the new at +0x1D0, +0x160 |= 0x20),
  starts limit k (0x140397A50(k, its data)) and pays MP (0x1402A5B60). **Checked (decomp)**
- **The limit modules.** 0x140397A50 → 0x14039EAD0 (Sonic Blade), 0x14039EEB0 (Ars Arcanum),
  0x14039FFD0 (Strike Raid), 0x14039F250 (Ragnarok), 0x1403A04A0 (Trinity Limit); 5 reads "still
  running", 6 stops it. Each registers a KH1 movement state of its own in KH1's state table
  (0x1402B1B80, table at 0x142DB3840, 0x40 a state) and puts Sora in it (0x1402B1960): Sonic Blade
  and Ars Arcanum state 0x20, Ragnarok 0x25, Strike Raid 0x27, Trinity 0x29. Their motions are
  requests on the swapped table (0x14029F980: 0x10000C8, 0x10000D2, 0x10000E6, 0x10000F0,
  0x10000FA). The state's enter sets actor flags 0x1200080 (Ragnarok 0x1200000); 0x1000000 is the
  script-hold bit the hook already knows, 0x200000 and 0x80 are not traced (**Assumption**: they
  take him out of control / hits). **Checked (decomp)**
- **Hits.** The modules land their hits with Sora's own attack rows (0x1402A3980, called from
  0x14039ECA0, 0x14039EDD0, 0x14039F100, 0x14039F7A0, 0x1403A0290). **Checked (decomp)**. That is the
  same path `kh1rows` already uses for KH2 moves.
- **The data.** `xl_limit.dat` (0xD00000 buffer) is loaded by the frame task 0x14028D620 while
  0x14028D5B0 allows it (field or battle mode, no limit, no event bits); state 0x14299C220
  (0 idle .. 5 loaded). Its header: +4 the motion table, +8 + 4k each limit's data. **Checked
  (decomp)**
- **The end.** 0x14028D620 clears bit 16 once 0x140397A50(5) reads 0 (bit 23 set first, the next
  frame clears both). **Checked (decomp)**
- **Trinity's partners.** Trinity's module keeps Sora (0x142F11548), 12 aura effects and a task
  (0x1403A10F0); how Donald and Goofy are moved in it was not traced.
- **What reads it.** Bit 16 of 0x142867364 is read by the offer, the data loader, the end task and
  the limit modules; event scripts were not checked for it. The motion-table swap means `kh2slots`
  must leave Sora alone while +0x1E0 is set (it already does: "never over another motion set").
  **Checked (code)**

## 4. Mapping KH2 limits onto KH1's party

| KH2 limit (file) | needs | KH1 partner |
|---|---|---|
| Trinity Limit (trinity) | Donald + Goofy | both, always in the party; KH1 has its own Trinity Limit (cmd 0x52) |
| Comet, Duck Flare (donald, donald2) | Donald | Donald |
| Whirli-Goof, Knocksmash (goofy, goofy2) | Goofy | Goofy |
| Session (aladdin) | Aladdin | KH1 guest in Agrabah |
| Applause Applause (jack) | Jack | KH1 guest in Halloween Town |
| Howling Moon (beast) | Beast | KH1 guest in Hollow Bastion |
| Peter Pan, Stitch, Chicken Little, Genie | summons in KH2 | Peter Pan is a KH1 guest; the rest are KH1 summons or absent |
| Auron, Mulan, Sparrow, Simba, Tron, Riku | KH2-only partners | none (Simba is a KH1 summon) |
| Sonic Rave, Last Arcanum, Strike Raid, Infinity | Limit Form | needs Limit Form on testing |

(KH1 party: Sora 0x142D37280, partners 0x142D37288 / 0x142D37290; a partner's member id is its
object's +0x4C, as 0x1403A1790 reads it. **Checked (decomp)**)

## 5. Plan (native: KH1's limit system running KH2's limits)

One system per version:

1. **KH2 limit commands in KH1's limit path.** New command ids for the KH2 limits, handled where
   KH1 handles its own (a hook on 0x14028D3D0): set bit 16 as 0x14028D800 does, swap Sora's motion
   table to one holding KH2's limit motions (baked as `kh2slots` bakes, on 0x14029E850), register
   a KH1 movement state of ours (0x1402B1B80, a free state id) and put him in it. KH1's offer
   gate, end task and event checks keep working because bit 16 is KH1's own.
2. **The limit's script.** Load the `limit/<lang>/<name>.bar` from the KH2 install and run its
   BDX with `kh2-bdx` from our state's update, traps answered on KH1's actors (KH2's trigger 22 at
   the start, trigger 23 for follow-ups, as 0x1403D8B40). Testing has none of this yet.
3. **Follow-ups on Triangle** through KH1's own box (0x140293000(3, command)), as KH1's modules
   do.
4. **Hits** through KH1's attack rows with KH2's ATKP numbers (the `kh1rows` path).
5. **The partner's part.** KH1's Donald / Goofy actors put in a limit state of ours playing
   KH2's partner motions retargeted to their KH1 skeletons. Nothing retargets the partners yet:
   the largest item.
6. **MP and the gauge.** KH2's "all MP" cost needs MP Charge (its own thread); KH2's limit time
   gauge would be a HUD element.

Performance: one hook on the dispatch, one state's update running a script only while a limit
runs, nothing per frame otherwise.

Not found / open: the partner's side in both games, KH2's limit timer, the meaning of KH1's
0x200000 / 0x80 actor flags and KH2's +0x124 bit 0x10000, the limit `.bar` contents (not opened).
