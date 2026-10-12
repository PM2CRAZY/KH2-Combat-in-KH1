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

## Several sessions at once

Up to one lead and three helper chats can work together. Follow `docs/WORKFLOW.md`: lanes
by crate, claims on the board in `local/board/`, delivery to the PC's `main` under a lock.

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

### 2026-10-09: playtest fixes, ledge grab, wall hits, Phase 3
- **Landing (exe):** landing hands over to the idle action. The stick takes over at once, and
  with a neutral stick Sora stops dead instead of sliding.
- **Arena:** a box top crossed during a fast fall now counts as ground, and the support and
  push-out radii match. This fixes slipping off box tops.
- **Ledge grab (exe):** the search box, timing and placement, using `hang_*` from prty/sstm
  and the type-4 radius. Motion 7 hangs; motion 8 climbs.
- **Wall hits (exe):** the BG-hit ATKP flag, the ray from the previous centre, the
  floor-check ray, and the recoil (motion 41/43, 50 push or hop). The swing counts as
  connected.
- **Reactions and clashes (exe):** the shared reaction routine (groups, motions, hit-back
  distance, launch physics), the 7×7 clash table with angle checks, and the party and enemy
  clash reactions.
- **Enemies:**
  - `kh2_data::enemy` loads the Shadow (M_EX010) from the install
  - `character_core::npc` runs it
  - the playground's Shadows attack on our timer, hit Sora and get guarded
- **Open:** enemy scripts, weight, revenge, Sora's damage-lock length.

### 2026-10-09: Phase 4 (HUD, menus, abilities, magic)
- **Playground changes:**
  - one Shadow by default (`--enemies N`)
  - the ledge hang waits for a fresh push or jump before climbing (our change)
  - Aerial Recovery (jump during a launch)
- **Enemy weight (exe):** the body push is split by objentry weight.
- **03system item/cmd tables and msg text:**
  - the command menu (Attack, Magic, Items, Drive)
  - the ability menu with real names and AP and the level's AP cap
  - HP/MP gauges
- **Correction (exe):** the ATKP row "level" byte is the defender's consecutive-hit count
  from the same attack group, not the attacker's level. It is now implemented with the
  game's per-object hit record (`combat_core::HitRecord`: hit-once bits, group/count, a
  30-frame group timer for the player, 4 re-hit interval slots). Sora's and the Shadows'
  swings use it too.
- **New crate `kh2-bdx`:** the BDX script machine, written from the interpreter. The trap
  names come from OpenKH's list, whose 828 argument counts match the executable.
- **New crate `kh2-magic`:**
  - spell casting (trigger 20 with caster, resources, sub and the `magi` record; trigger 21
    at the cast motion's magic-cast event)
  - the 59 traps the six spells use
  - spell attacks (the game's magic attack shape and sweep, duplicates sharing ids)
  - effects (PAX elements by id, bone codes, lifetimes from the DPD emitter headers)
  - a y-down/y-up flip at the boundary
- **Reaction 5 (exe):** Magnet's orbit pull.
- **Verified:**
  - every spell file at every level runs to the end headlessly (`kh2-magic` test with
    extracted files)
  - in the playground: Firaga hits 5 times (alternating 8/10-frame re-hits, a 175% final);
    Blizzaga's shard; Thundaga's bolt; Curaga heals; Magnega lifts the Shadow into an orbit;
    Reflega stops a swipe and counters with 574 ×5 and a 667 burst
- **Open:**
  - the MP Charge rate
  - the target search without lock-on
  - Drive Form variants
  - enemy scripts (the VM is ready)

### 2026-10-09: Phase 4 finished (map collision, camera, Drive Forms, reaction commands)
- **New crate `map-collision` (exe):** the game's background solver.
  - one sphere (r 90, centre 115) plus a lower sphere for walls
  - sub-stepped push-out: faces, then edges
  - anti-tunnelling segment test
  - the floor ray with a 24-unit step-down
  - the 49% ceiling rule with a 16-frame air-command lock
  - Step-up 25 and the 38.5° slope limit come from the sphere's geometry.
  - The arena is now polygons: stairs, ramps, a floating slab.
- **New crate `kh2-camera` (exe):**
  - the field camera: distance/height curve, swing-behind, spring smoothing, wall pull-in
  - the lock-on framing
  - target switching
- **Drive Forms (exe + data):**
  - each form's object from objentry/pref/PTYA/wmst, with two weapons where the data has them
  - `character_core::drive`: the gauges, costs, Anti roll, levels and growth
  - Sora's Drive motion and invulnerability, the arrival motion, revert, magic subs
  - Wisdom's shots from its model's BDX script (`Magic::object_effect`, trap
    `obj_effect_start`)
- **Reaction commands (exe + data):**
  - RC areas from type-5 collision with group switching (motion trigger 4)
  - priority/score selection, the low-HP Drive offer
  - capture motions read from nested motion sets
  - the Soldier's Cyclone, as its script runs it
- **Correction:** the "Shadow" was a Soldier (M_EX010). The real Shadow is M_EX020.
- **Open:**
  - enemy scripts
  - other enemies' RCs
  - items, form keyblade stats
  - the MP Charge rate

### 2026-10-12: KH1 port started (KH2 melee in Kingdom Hearts Final Mix PC)
- The user's request: KH1 stays KH1 except the melee combat, which becomes the KH2 rewrite's;
  KH2's abilities and combo modifiers go in KH1's Ability menu. The user's choices: KH1's own
  melee abilities are swapped for their closest KH2 counterparts in the same menu slot and
  level-up spot (the extra KH2 modifiers are added as new abilities); KH2 modifiers are learnt
  through KH1's level-up tables (Warrior / Guardian / Mystic) and equipped with AP.
- New crate `kh1-data`: KH1's packages (`kh1_first..fifth`, same format and cipher as KH2's,
  key table from the KH1 executable; `BackupImage/` originals first so a randomizer's edits
  don't leak in). CLI `kh1-extract --kh1 <install> --file a,b --out <dir>`. All 601 names in
  `kh1_first` resolve from OpenKH's `kh1idx.txt` / `kh1pc.txt` lists (the user's install).
- Sora is `xa_ex_0010.mdls` (8 sections; the first is `MOBJ`) and `xa_ex_0010.mset` (3
  sections; the first is `MMTN`, a motion table); battle table `btltbl.bin` (ability level
  tables at `+0x3BF8` and two more, per the Archipelago scripts).
- The KH1 executable is x64 and imports `DINPUT8.dll` (a proxy DLL; the
  folder is shared with KH2, BBS and Re:CoM, so the DLL must only act in KH1). The user runs
  OpenKH's Panacea (`DBGHELP.dll`) and LuaBackend with an Archipelago randomizer: stay
  compatible.
- Scope widened (the user, same day): KH2's movement abilities too, and KH1 Sora uses KH2's
  animations for everything except summons, Drive Forms (none in KH1) and magic. So KH2's
  controller drives Sora outright (idle, run, jump, glide, dodge, guard, hurt, attacks), as
  in KH2's combat mode, with KH1's collision underneath; KH1 keeps magic, summons, items,
  cutscenes and menus. Proposed (to confirm in play): KH1's story abilities map onto KH2
  growth (High Jump x1-3 -> High Jump LV1-3, Glide -> Glide LV1-2, Superglide -> Glide
  LV3/MAX, Dodge Roll -> Dodge Roll LV1); Quick Run, Aerial Dodge and the higher growth
  levels go into the level tables; Mermaid Kick and KH1's special attacks (Sonic Blade, Ars
  Arcanum, Ragnarok, Strike Raid) keep KH1's animations, having no KH2 counterpart.
- KH1 Sora's skeleton (`kh1_data::mdls`, read from the user's `xa_ex_0010.mdls`): 293 bones,
  0x30 bytes each (KH2's MDLX bone packed tighter: scale, Euler rotation, translation, links
  parent / sibling / child in 10-bit fields), bind pose a T-pose like KH2 Sora's (256 bones;
  KH1 Sora's hips at 78.9 against KH2's 88.5). `examples/skeletons.rs` prints both bind poses
  side by side for the retarget map.
- New crate `kh1-hook` (64-bit `dinput8.dll`, `-Zbuild-std` for `x86_64-pc-windows-gnu`):
  stage 1 only forwards DirectInput, and in KH1 alone logs a heartbeat to `kh1-kh2.log`
  (which other mods are loaded). Installed in the KH 1.5+2.5 folder.
- KH1's skeleton pass (Ghidra, the user's Epic exe; ASLR, so base + RVA): `0x140391E20`
  `(world, instance, time f32, motion)` samples the motion into per-bone arrays and builds
  the bone matrices (`0x140390260`); `0x140392710` `(world, instance, flags)` is the variant
  with ground IK (called by `0x14029E490` with the actor's instance at `actor+0x15C`). The
  instance: `+0` the model header (bone count, bone-table handle); handles to 0x40-byte
  per-bone matrix arrays at `+0x1B0..+0x1C0`, translation / rotation arrays at
  `+0x1C4` / `+0x1C8`. Game handles are 32-bit: `[exe+0x2EE4730][(h & 0x7FFFFFFF) >> 25] |
  (h & 0x1FFFFFF)` (`0x14038B230`). Matrices are D3D row-vector layout (translation at
  +0x30), i.e. glam's column-major memory.
- `kh1-hook` stage 2 (probe): inline hooks on both (x64: `jmp rel32` to a stub near the exe,
  trampoline with the moved prologue); logs Sora's (293 bones) arrays; Insert cycles a
  T-pose override through the five arrays to learn which one the game draws.

