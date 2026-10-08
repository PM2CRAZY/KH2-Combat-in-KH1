# AGENTS.md

Notes for anyone (human or AI agent) working on this repo.

## Ground rules

- Never commit game files, extracted tables, dumps, Ghidra databases or decompiled code. The
  `.gitignore` is a whitelist: add a pattern only for source, docs or manifests.
- Put scratch output (table dumps, binaries) in `local/`. It is ignored.
- `combat-core` must stay free of KH2 knowledge and engine dependencies. KH2-specific
  mapping belongs in `kh2-data`.
- Write down every rule's source and confidence in `docs/combat-model.md`.
- When code is ported from another project, credit it in `NOTICE` and `CREDITS.md`
  and keep its licence.

## Testing against the game

```
KH2_GAME="D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe" cargo test
```

## Development log

### 2026-10-07: milestone 1
- Surveyed existing work: OpenKH (format docs and C# readers), GovanifY/kh2 (PS2 decomp, early
  stage), KHPCPatchManager. Chose documented formats; no decompiling was needed.
- Ported the HED/PKG reader. The cipher's substitution table is located in the user's exe at
  runtime by a sliding-window permutation search, then checked against a SHA-256.
- Parsed 00battle.bin. The counts on the user's install match OpenKH's docs (atkp 2713, enmp
  229, ...). Live and backup copies are identical.
- Derived from the data:
  - ATKP power is a percentage (most common value 100; Thunder spells use 375 = ×3.75)
  - the LVPM level-99 Defense of 51 matches the community example
  - ENMP min/max damage × HP is nearly constant, which suggests a fraction of HP
- Open: the clamp reading, critical_adjust scope, element ids 4–6. Next: an MSET/ANB motion
  trigger reader to get hitbox timing and Sora's combo attack ids.

### 2026-10-07: milestone 2 (movement + combat playground)
- Added readers for 03system.bin `pref` (prty/fmab/plyr/sstm), PTYA combo rows, MSET/ANB motion
  headers, root-motion F-curves, motion triggers and model collision.
- Findings on the user's install:
  - trigger frames are 60 Hz and relative to the clip start
  - Sora's keyblade hitbox windows live in `W_EX010.mset` at the same slots as `P_EX100.mset`
  - root motion: Dodge Roll 438 units, A300 lunge 92
  - fmab records 0–3 = growth LV1–MAX
- Built `character-core` (engine-agnostic controller) and `kh2-bevy` (capsule playground).
  The playground was verified under Xvfb with `--synthetic --autoplay --screenshot N`. The
  real-data path was verified on the user's machine with `kh2-combat sora`.
- Open: gravity (estimated), the combo `sub` chain, skeleton-accurate hitboxes, enemy attacks.
