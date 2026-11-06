# KH1 on Epic: how the mod's Steam addresses are found there (testing-merge, tm41)

The owner (chat 2026-11-06): "Let's try to make this epic compatible as well". He tests on Epic
himself.

## The two builds

- Steam: `KINGDOM HEARTS FINAL MIX.exe`, link stamp 1721645006, SHA-256 `d7907462...`.
- Epic: link stamp 1721648200, SHA-256 `082b4349...`, 7,491,856 bytes. Checked (exe).
- Same game, compiled the same day; each links its own store's code, so functions and globals sit
  at different places. 13,048 of Epic's 15,824 functions are byte-identical to Steam's once
  relative displacements are left out, and 318 more pair up through the calls of matched code.
  Checked (exe, `tools/epic/match.py`).

## How an address is mapped (`tools/epic/map.py`)

Every Steam address in the source is written `St(0x...)`. The script collects them and finds each
one in Epic:

1. **Code**: by the function it's in (identical code, or a paired function up to where the two
   first differ), at the same offset.
2. **Globals**: by the RIP-relative operands of matched code that reach it (votes; the majority).
3. **Globals no code reaches** (a field of a table read through its base): from the matched globals
   on both sides of it, when both are shifted by the same amount. Assumption (exe): such a table
   moved as one block.
4. **Code in a function that wasn't paired** (small functions with no unwind data): its bytes are
   searched near where its neighbours moved to, with displacements left open; it must be found
   exactly once, or once at a neighbour's exact shift.
5. **By hand**: the few the script can't place (`MANUAL` in the script, each with how it was
   checked).

The old table `win/steam_map.in` (Epic → Steam, for the older `Rva` addresses) agrees with all 308
of its pairs the script also maps on its own; none differ. Checked (exe).

At run time `base + st` looks the address up in `win/epic_map.in` on Epic. An address made by adding
to a mapped one (`St + n`, a field or row) keeps that distance from it.

## Where the builds differ in more than addresses

- **KH1's settings block** (Steam `0x1404D8614`, returned by the getter `0x14012AF20`; Epic's getter
  `0x14012AC50` returns `0x1422DB014`): Epic's has no first 4 bytes of Steam's. Every field KH1's
  own code reads right after calling the getter is 4 lower on Epic (0x16 → 0x12, 0x1E → 0x1A, 0x24 →
  0x20, ...; 170 call sites compared). Checked (exe). So `St(0x4D8614)` maps to Epic `0x22DB010`, and
  `SETTINGS + field` with Steam's field lands on Epic's. Volume Mute's rewritten reads use the field
  4 lower on Epic (`volmute.rs`, `settings_head`).
- **Byte checks of RIP-relative code** (a hook's expected first bytes that hold a displacement):
  Epic's bytes are given beside Steam's and chosen with `addr::pick` (chest's done push, the
  Customize-time normal, the skip function, the KH2 list's three functions, Limit Form's provider set
  and register, the hit record leas). Checked (exe): 185 byte checks compared; these 11 differed, all
  in a displacement only.
- **Absolute table offsets** (`[r14 + r*4 + 0x528BE0]`, image-base addressing, as the Drive orbs'
  prize table reads): Epic's code has Epic's offset; the mod compares against the mapped table, so it
  needs nothing more.

## What isn't checked

- Nothing has run on Epic yet. The tester's log on Epic is the check: the hooks log what they hooked
  and any "unexpected bytes".
- Tables placed by their neighbours (step 3) are an Assumption until a feature reading them works
  in game.
- Frame Advance's `PUT_BACK` block (`0x1427F13D0`..`0x142866D18`) is copied by its Steam length.
  Assumption (exe): the region moved as one (both ends map with the same shift).
