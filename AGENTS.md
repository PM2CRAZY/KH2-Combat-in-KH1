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
