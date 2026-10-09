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

## Reverse-engineering notes

- Ghidra runs headless on a private copy of the user's executable, outside the repo. Commit
  only plain-language findings (in `docs/`) and our own Rust. Never commit decompiler output,
  Ghidra projects or byte dumps.

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

### 2026-10-08: Phase 1 from the executable
- Ghidra 12.1 headless on the user's exe. MSVC RTTI names the action classes, so the
  action vtables (jump, glide, air slide, hop, idle) were easy to find.
- Implemented from plain-language notes:
  - the 30 Hz velocity tick
  - the double-sine jump with gravity 20/49
  - air control 0.9/0.95
  - glide, Quick Run and Aerial Dodge formulas
  - the PTYA selector (table order, `sub` anti-repeat, kind/flag semantics, `combo_offset`)
  - the hit-driven combo counter
  - jump-attack arcs and the lunge factor
  - the damage routine (stat source byte, base-term clamps, ceil rounding, difficulty table,
    boosts, Cure)
  - enemy status init
  - ATKP level variants
- Removed: estimated gravity, the approach-assist hack, the `--clamp` and `--power-scale`
  switches, and the score-based selection.
- Verified on the install:
  - the jump peaks at 185 on frame 33
  - whiffing alternates 151/153
  - connecting gives 151→153→155
- Open: jump hold, stick-tilt mapping, landing, cancel points, auto-target, timer units.

### 2026-10-08: skeleton, animation and a mannequin
- Aerial Dodge is the jump button in mid-air (KH2), then Glide; it was on the action button.
- New readers:
  - MDLX skeletons
  - full interpolated motions (initial pose, F-curves with pre/post behaviour, IK helpers,
    joint table, constraints, activations, limiters, expressions, external effectors)
  - `00objentry.bin`
  - `03system.bin` → `sklt` (weapon bones)
- New crate `kh2-anim`, implemented from plain-language notes on the exe's motion evaluator:
  - key times are 60 Hz
  - the unscaled/scaled matrix pair
  - constraints and limiters
  - one- and two-bone analytic IK
  - the expression language
- Root motion is now sampled at 60 Hz key time. Lunges were half length before: A300 now
  travels 150.
- Playground: Sora is a capsule mannequin on his real bones, playing his real motions, with
  the Keyblade attached to bone 178. Hits use the real bone-attached attack spheres.
  `--data-dir local/re` runs from extracted files (research only).
- Verified locally:
  - every Sora motion poses without NaNs
  - idle stands at y = 0
  - screenshots of idle, run, attack, jump, roll and glide look right
- Open: per-move blend times, foot ground adaptation, head look-at, the Shadow's own
  skeleton.

### 2026-10-08: playtest fixes
- Stick left/right was mirrored (screen-right is −X when looking along +Z).
- Read from the exe:
  - **Variable jump height:** an early release swaps the rest of the rise for a 5.1-unit
    hop, after 6 frames and before apex − 5.
  - **Quick Run** lasts only while the button is held, and is ground-only.
  - **Dodge Roll's** frame-10 event chains into Quick Run while the button is held.
- Air combos: whiffs now advance the count, per the user's knowledge of the game (the exe's
  rule for this isn't found yet; ground stays hit-only).
- Rig:
  - **End-of-action zoom fixed:** the last frame was posed as a free loop without the root
    offset, and blending then smeared that jump.
  - Bones are now kept in object space.
  - The Keyblade's blade/keychain orientation is fixed.
  - New simplified mannequin.

### 2026-10-09: Phase 2 (skeleton-accurate hitboxes)
- **Hit detection, from the exe:**
  - Attack windows collect the attack- and guard-type shapes of their collision group.
    Defenders use their hit-type shapes.
  - The overlap test: sphere, column, ellipsoid; cubes never hit.
  - The 1.5 × radius sweep between frames.
  - Implemented engine-agnostically as `combat_core::{Shape, AttackVolume, first_overlap}`,
    with unit tests.
- **Data:**
  - `kh2_data::sora::BoneShape` keeps every MDLX collision entry: type, kind, bone or root,
    offset, lift, radius, height.
  - Guard windows keep their group (`WindowKind::Guard { attack, group, weapon }`).
    `Output::guard_boxes` lists them.
- **Blending, from the exe:**
  - blend lengths of 8 (default and locomotion), 16 (glide and the fall helper), PTYA
    `blend_time` for attacks (0) and 0 for queued follow-ups
  - the clock starts at 2
  - walk↔run phase sync
- **Playground:**
  - hits use the game's test, with the sweep
  - F1 attack/guard, F2 hit, F3 skeleton lines, F4 other collision types
  - `--show-hurt`, `--show-skeleton` and `--show-other` for screenshots
- **Open:**
  - clash table
  - wall-hit ray
  - the motion freeze during a blend
  - group-enable bits from the collision header
  - enemy collision (Phase 3)
