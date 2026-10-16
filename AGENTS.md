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

- Loading (the user: KH1 crashed launched directly; then the Epic launcher would not start
  with our `dinput8.dll` in the shared folder): launched without Epic's arguments KH1 crashes
  on its own (`-epiclocale` read as null in `0x1400D9BD0`). The proxy is out of the game
  folder. The DLL is now loaded by Panacea (OpenKH's `DBGHELP.dll`, read in Ghidra): in KH1
  only, it `LoadLibrary`s every `*.dll` in `<mod_path>\kh1\dll\` and calls `OnInit(wchar_t*
  mod folder)` / `OnFrame()` if exported (`kh1-hook` exports `OnInit`, which logs the route).
  Installed as an OpenKH mod (`crates/kh1-hook/openkh/mod.yml`: `dll/kh1-kh2.dll`, method
  copy) in the user's `mods/kh1/kh2-combat`, enabled in `mods-KH1.txt`, plus a copy in the
  built `mod/kh1/dll/` so it works before the next Mods Manager build.
- Probe fix (the user: Insert did nothing; log: loaded by Panacea, hooks in, but Sora never
  seen): the instance's `+0` points at a slot that points at the model header, whose bone
  count is a u16 (`0x140390260`: `**(short **)*instance`); we read one level short. Every
  model size posed is now logged once. `+0x1B0` holds the sampled channels (scale `+0`,
  rotation `+0x10`, translation `+0x20` per 0x40-byte bone), not matrices; `+0x1B8` the built
  matrices; the T-pose test only writes arrays whose bones look affine.
- Deploying `kh1-hook`: while KH1 runs, Panacea's copy in `mod/kh1/dll/` is locked and a
  write to it does not land (the upload reported success; the second test ran the old
  build). Rename the loaded file aside (`*.dll.loaded-old`, not loaded: Panacea takes
  `*.dll` only), copy the new one in, and check its SHA-256 on the PC after every deploy.
  The user's two crash dumps of 2026-10-13 (KH1 and the launcher, the launcher without our
  DLL) are both the null `memcpy` of a start without Epic's arguments.
- Stage 2 result (the user's Insert test, log): Sora is the 293-bone model posed by the plain
  pass (also by the ground-IK pass). Writing into `+0x1B4` gave a clean T-pose: that is the
  world matrix array the game draws (y-down world). `+0x1B0` holds the sampled channels (scale
  `+0`, accumulated scale `+0x10`, Euler `+0x20`, translation `+0x30` per 0x40-byte bone), kept
  between frames (writing it made him huge); `+0x1B8` is each bone's matrix relative to its
  parent (bone 3's matches its channels with KH2's Euler order). The T-pose lay sideways
  because it was placed from the bind's root rotation; in motion KH1's root is the hips with a
  yaw. Model → world is now `+0x1B4[0] * +0x1B8[0]⁻¹`, read each frame.
- New `kh1_data::retarget` (ours, from the two bind poses): KH2 Sora's pose on KH1 Sora's
  skeleton. Both bind in a T-pose facing +z with the same chain layout; 113 KH1 bones take
  their KH2 counterpart's model-space turn from rest (legs, spine, neck, head, arms, fingers),
  the rest turn with their parent; KH1's bone lengths, hips scaled by the hip-height ratio.
  `examples/retarget_check.rs` (KH2=1 for KH2's own pose) checked idle and run against KH2's
  own poses in side / front plots.
- `kh1-hook` stage 3: KH2 Sora loads on a thread from the KH2 install in the same folder
  (backup originals if the Mods Manager made them); Insert cycles off / KH1 T-pose upright /
  KH2 idle / KH2 run, written into `+0x1B4` after the game's pass (the Keyblade still follows
  KH1's hand). Logs a check that `+0x1B8` chains to `+0x1B4`.
- Waist fix (the user's screenshot of the in-game idle: the waist contorted). KH2's root,
  pelvis, spine, first hip bone, ankles and arm bones carry another built-in orientation in
  its motions than in its bind pose (the idle's pelvis 210° from rest, ankles ~115° off their
  foot every frame), so copying their turns from rest spun KH1's belt, pouches and shorts.
  `kh1_data::retarget` now measures every turn from KH2's joint positions: torso frames from
  bones riding on each part, limb segments aimed joint to joint and rolled by the knee /
  elbow direction (foot / knuckle line while nearly straight), fingers by segment.
  `examples/roll_probe.rs` checks it (leg rolls within ~30° of the foot in the idle; the run's
  kicked-back thigh 69° from the pelvis where the foot-rolled version gave 166°).
- KH1 motions (`kh1_data::mset`, from the user's `xa_ex_0010.mset` and the skeleton pass
  `0x140391E20` / channel evaluator `0x14038F9D0`): the `MMTN` table (118 slots for Sora),
  per motion frames / fps / bones / channels / keys, keys `time<<8 | interpolation` (1/256
  frames; hold, linear, Hermite with per-frame tangents). KH1 animates bodies through ~11 IK
  helpers per motion (ground root, body centre, hips with their yaw, pelvis, chest, hands,
  feet; records of scale / parent / rotation / translation) and per-bone flags pick the IK
  chains and their target helpers, so plain channel playback leaves the legs straight. Slots
  read from the helper paths: 0-1 standing, 2-4 walking (?), 5-7 running (looping), 8-10
  jumping / in the air, 11 landing; to be confirmed in play. A motion's counts (frames,
  channels, helper channels, helpers) name its slot (116 signatures for 118 slots; the shared
  pairs 77/78 and 86/87 are not movement).
- `kh1-hook` stage 4: the motion playing is at `instance + 8`; standing / walking / running /
  in-air / landing slots get KH2's idle / walk / run / jump or fall / land (8-frame blends),
  every other slot keeps KH1's animation. Insert cycles follow / off / T-pose / idle / run.
  Slot changes are logged with speed and climb.
- Stage 5 (the user: a split-second vanish going idle → walk/run; requests: jump, glide and
  attack animations, Keyblade and no-Keyblade versions in their situations). Play log
  (2026-10-13): slots 0 / 2 / 5 field idle / walk / run, 1 / 3 / 6 the battle ones (the field
  ones follow attacks back to the field); jump 12 → 13 → 10 → 11; glide 102 / 103 / 104;
  ground combo 98 → 99 → 98 → 101. `pose::play_for` maps them: field slots take KH2's field
  variants (no Keyblade; KH2 has them for idle / walk / run), jump / fall / land, glide / glide
  end, and swings 151 / 152 alternating with finisher 154 (KH2's no-target ground rows),
  each stretched to the KH1 swing's length and with KH2's lunge taken out (KH1 moves him).
  The vanish: some passes leave `+0x1B4[0]` as we wrote it, and model → world read from it
  compounded our own pose (the log's speeds of 10^4-10^5 units/s too); the last placement
  is kept when the root reads back as ours. The Keyblade: a small model (≤ 40 bones) whose
  world matrix holds a fixed relation to KH1's hand over 20 frames while the hand moves is
  taken as held and re-placed from the hand drawn (logged; lets go if the relation breaks).
  `examples/kh1_motions.rs` / `kh2_rows.rs` list both games' motions (scratch research).
- Stage 6 (the user: the vanish still there; animations jumbled; air combos glide; sliding
  in every animation). Log: 102 / 103 / 104 are the air combo (two swings and a finisher),
  not the glide (the user has no glide yet); now KH2's 183 / 184 alternating and 181. The
  vanish: speeds of 10^4 units/s still at every slot change, so KH1's blends between motions
  mix our drawn pose back into `+0x1B4[0]`; placement now comes from the pass's own world
  matrix (y and z columns negated, axes normalised, as `0x140391E20` does with `× -1` at
  `0x140430938`) once it agrees with the bone-derived one for 30 frames (logged either way),
  and meanwhile a jump of over 30 units in one frame is ignored unless it persists 3 frames.
  Sliding: walk / run play at his measured speed over the loop's ground speed (planted heel
  against bone 0: KH2 run 484 units/s, walk 111, field run 427, field walk 134; KH2's own run
  speed 8 / frame = 480 / s agrees), scaled by the hip ratio. Swings still slide with KH1's
  own lunge until KH2's controller moves him. Combo alternation no longer resets on the idle
  KH1 plays between swings (1.2 s gap).
- Direction change (the user, 2026-10-13: "I want the EXACT KH2 combat, not just
  animations"; sliding stayed because KH1 still moved him). KH2's controller
  (`character-core`) will drive Sora: input, movement, physics,
  jump / glide / dodge / guard, the combo system and hit timing; KH1 keeps its world,
  collision, enemies, HP / MP, magic, items, summons, menus. The animation-follow mode
  (`pose::follow`) stays as the fallback until then. Stage 6 log: placement from the pass's
  matrix took; glide is slots 75 / 76 / 77; 117 unknown.
- Ghidra findings for the takeover:
  - Input: every controller type ends in `0x140112C70` `(device state, PADDATA out, config)`,
    KH1's PS2-style pad record: `+0` buttons u16 in PS2 order (bit 0 Select ... 4-7 d-pad
    Up / Right / Down / Left, 8-11 L2 R2 L1 R1, 12-15 Triangle Circle Cross Square; it clears
    opposite d-pad pairs), `+2..+5` stick bytes (order to confirm), `+0x1F` status. XInput is
    imported by ordinal (2 GetState at `0x140113B80`), DualShock / DualSense through HID
    (`0x1401132A0`, `0x140111490`).
  - Floor query `0x1401910E0` `(query)`: `{x, y, z, 1, 0, range, 0, 1}` in, nearest level
    surface vertically within range at `+0x20` (point), `+0x30` face (short, -1 none)
    (`0x140190B00` walks the cells' polygons); used by the ground-IK pass.
  - Actor animation update `0x14029CF10` `(actor, dt)`: `+0x10` position (vec4, to confirm),
    `+0xD0` world matrix, `+0x15C` animation instance handle, `+0x168` motion slot u16,
    `+0x16C` motion time, `+0x1D4` motion table handle.
- `kh1-hook` stage 7 (probe, `probe.rs`): finds Sora's actor through his instance, searches it
  for his drawn position, puts hardware write watchpoints (debug registers on every other
  thread, a vectored handler) on that field for 25 s and logs the writing code by address;
  logs the pad record on change.
- Stage 7 probe result (the user's run): Sora's actor at `0x142D37C80` (party array, Sora
  first; actors are 0x4B0-byte records in a pool from `0x142D37CA0`); position `+0x10`
  (copies at `+0x50`, `+0x100` in the world matrix at `+0xD0`, `+0x260`, `+0x2E0`, `+0x320`);
  the only regular writer is `0x140386D00` (vector add) from the collision mover
  `0x140299350`, ~57 times a second. Pad record: Cross 0x4000, Circle 0x2000, Square 0x8000,
  d-pad as PS2; stick bytes rx, ry, lx, ly (left: 0 = left / up, 0xFF = right / down).
- Ghidra, the actor's frame (`0x140295E50`): think (`0x1402957F0` picks it by object kind;
  `0x1402A1170` for the player and party: the move vector, the state's handler from the
  table `0x142DB4240` (0x40 bytes a state: flags, enter, exit, update, displacement,
  transition, ...; registered by `0x1402AF9F0` from `0x1402AF8B0`), the turn toward `+0x150`),
  physics `0x140298230` (state displacement, platform carry `+0x120`, `0x140297710` move with
  collision, floor `+0x320` / height above it `+0x3D4`, transition), matrix, animation.
  `0x14298D3A0` is set while an event runs. The state at `+0x70` stays 0 through KH1's
  attacks, so KH1's own actions are told by the motion slot instead.
- `kh1-hook` stage 8 (`takeover.rs`): KH2's controller drives Sora's movement. Hooked
  physics step; for Sora, with no event and KH1 showing standing / walking / running (slots
  0-7) or KH2 having him in the air: input (direction from KH1's camera-relative move vector
  `+0x20`, tilt from the left stick, Circle jump, Square KH2's action button), KH2's step
  with KH1's height above floor, KH2's displacement through KH1's mover `0x140297710`
  (stepping down up to 24 onto lower floor on the ground, put back on the floor when below
  it), facing written to `+0x30..+0x3C` and `+0x150`, KH2's animation player for the pose
  (field variants on KH1's field slots). KH1 doesn't see Circle / Square while KH2 has him.
  Every movement ability at MAX for testing. Insert: KH2 controls / KH2 animations on KH1 /
  off / T-pose / idle / run. Cross still goes to KH1 (talk, chests, menus, KH1's attacks,
  which hand control back to KH1 until they end); KH2's combo system is next.
- Stage 8 result (the user + log): KH2 drives walk, run, dodge roll, quick run, aerial dodge
  and glide (300-500 units up off ledges); the jump played but didn't rise: KH1's mover snaps
  a grounded actor back onto the floor each frame, undoing the jump's first small rises.
  Fix (8b): only the sideways move goes through the mover; in the air (or rising) his height
  is KH2's, landing on the floor the mover reports (`+0x324`); on the ground the mover's
  height stands, stepping down onto a floor up to 24 below, else he falls. `+0x3D4` is
  rewritten for KH2's next step. The user's next requirement: attacks 1:1 with KH2.
- Phase 3 (combat parity) started; the user: movement looks great. Ghidra:
  - KH1's hit pipeline: `0x1402BD180` `(attack, target)` (an attack touched an actor: checks
    the target can be hit `0x14028FEE0`, queues a hit record, MP for the attacker from the
    attack data `+0x2C`, the attack's hit script) → `0x1402BD480` `(attack, hit-once bit,
    target)` takes a 0x40-byte record from the pools at `0x142DE6810` / `0x142DE7810` and
    links it on the target (`+0x3EC`, hit-once bits `+0x3E0`): attacker `+4`, damage `+8`,
    flags `+0xC`, knock-back direction `+0x10`, attack data `+0x2C`; the target's think pops
    it (`0x1402BCFB0`) and applies damage and reaction.
  - KH1's damage `0x1402BD8D0` `(attack, target, flags)`: power = attack data `+0x14` +
    attack `+0x2C`, critical (chance `+0x94`, bonus `+0x98`), against the target's defence
    (stats `+0x6C` → `+0x50`) through the table `0x142DE67C0`, min / max from the attack data
    (`+0x20` / `+0x24`) scaled by attack `+0xAC`, clamps from the target's stats (`+0xC0` /
    `+0xC2`), element resistance (stats `+0x78 + element * 4`, element = attack `+0x30` bits
    6-9). Attack objects: owner handle `+0x34`, attack data handle `+0x5C`, hit-once bit
    `+0x54`.
  - KH1's collision pass `0x1402B1A50` tests 0x40-byte entries (owner, attack object,
    shape at `+0x20`, overlap `0x1402B3D50`); attack vs body → `0x1402B1ED0` →
    `0x1402BD180`; attack vs attack → `0x1402B2A30` (clash, priority `+0x68`).
  - Battle mode: bit 0 of `0x142867CD8` (picks field / battle idle-walk-run in
    `0x1402C9FF0`). Actor iterator `0x14028F990` `(previous) -> next`. Actor kind byte `+6`
    (3 enemy AI, 4 / 5 / 9 / 10 objects), body radius `[[+0x134] +0x14] +0xC`.
- `kh1-hook` stage 9 (combat probe): set-motion hooks log the code putting Sora into
  non-movement slots (stack return addresses), the attack-touched-actor hook dumps Sora's
  attack records / targets' stats and keeps the last as a template, the actors are listed
  every 4 s. Fixed before shipping: `0x14029D630`'s third argument is a float (XMM2).
- `kh1-hook` stage 10 (combat parity, `win/combat.rs`), from the stage 9 log and Ghidra:
  - The Attack command `0x1402A5380` (command type 1 in `0x142D54440`) runs a row of the
    actor's attack table with `0x1402A4760` `(actor, row, dir)`; `0x1402A17F0` reads the row
    (`0x142D5D4B0[object id] + row * 0x14`: motion request `+0`, attack data
    `0x142D23740 + [+4]`, effect / sound bytes `+0xC..+0xE` → attack `+0xA8..+0xAA`, flag
    `0x400000`) and makes the attack object with `0x1402B10E0` `(actor, hit bit from
    0x1402B15D0, data)` (pool `0x142DB5110`, 0x40 × 0xC0; freed by `0x1402B13D0`).
  - Per frame `0x1402B22D0` builds the body entries: hittable actors (`0x14028FEE0`), team
    `(+0x374 >> 5) & 3`, parts at `[[+0x134] +0x18]` (0x14 bytes each, count byte `+6`,
    disabled bits `+0x3A8`), shape by `0x1402B3CB0` (type, radius, half height, centre `+0x10`).
    Contact `0x1402B1ED0` `(attack entry rcx, body entry rdx)` does the record, its push
    direction, the hit effect and the Keyblade hit sound (`0x140293FD0`).
  - While KH2 has Sora, a type-1 Attack command is swallowed and becomes KH2's attack press.
    KH2's swept attack shapes and hit records test KH1's body parts;
    each effective hit is landed by a fresh attack object from Sora's own row (ground / air ×
    swing / finisher = KH1 motions 200 / 202 / 203 / 205), hit-once bit cleared during the
    call, then freed. `probe::hit` scales the record's damage by the KH2 row's power.
- `kh1-hook` stage 11: the user's stage 10 log: KH2's hits land (31-72 on the ground); the
  Attack command's air rows are 204 / 205 (203's first row, 0x4C58, hit for 95-144). Area
  scripts (`0x14028C550`, per-actor action list at `+0x440`, pool `0x142BA1E80`, 0x60 each,
  kind byte `+0x10`, next `+0x58`) walk Sora through doors / zone entries: KH2 lets go and
  `pose` shows KH1's own animation while the list is non-empty. Diagnostics for the
  Keyblade (invisible during KH2's attacks): its instance is combat state `+0x18`; where it
  is posed is logged with each KH2 state change.
- `kh1-hook` stage 12: the user: the game crashed going through a door (stage 11; no dump,
  last line KH2 retaking Sora between the door's turn and its walk, actor flags `0x1000000`
  set). Actor flag `0x1000000` (script commands 0x24 set / 0x23 clear) now also keeps KH2
  off (set from a door's turn to the end of its walk, and at a zone entry). `win/crash.rs`
  logs the first fatal exceptions (code, address, registers, return addresses). The
  Keyblade instance is never posed by the two skeleton passes; its own matrices are logged.
- `kh1-hook` stage 13: the crash note found the door crash: ours, in the stage 9 probe's
  set-motion logging (`probe::callers` scanned 0x800 bytes above its frame and ran off the
  top of the game thread's stack when a door script set Sora's exit motion). The scan now
  stops at an unreadable page. The Keyblade object (combat state `+0x18`) is no model
  instance (`[0x82, 0, 0, 9, handle, ...]`); its model instance is the handle at `+0x10`.
- `kh1-hook` stage 14: the user: doors work; the Keyblade is still invisible while
  attacking. Ruled out offline (`kh1-data` example `hand_check`): no KH2 attack motion
  degenerates or stretches the retargeted right hand or the 40 bones under it (101-127,
  280-292). The combat state's `+0x18` object is no model either: it is a table (count `+0xC`,
  0x20-byte entries from `+0x10`, looked up by `0x1401DDE80`), the hit-effect points. Probe:
  every small model (≤ 40 bones) posed near Sora is logged with where its placement matrix
  comes from (a bone of Sora's drawn matrices, if it points into them), at each KH2 state
  change and every 15 frames of an attack.
- `kh1-hook` stage 15: the stage 14 log: no small model is posed near Sora's hand at any
  time (none of the passes poses a Keyblade model). KH1 Sora's own skeleton carries the
  weapon: bone 280 under the right hand (grip, 4 units from it), 281-282 at the same point,
  283-292 a chain of short segments (flags 1) reaching 56 units (`kh1-data` example
  `bones_under`). The probe now logs bones 280 / 283 as KH1 posed them and as we drew them
  (position, determinant, distance from the hand). The weapon-object probe is gone.
- `kh1-hook` stage 16: the user's video (Traverse Town, no enemies): Sora's hands are always
  empty; the weapon bones are drawn in his hand (stage 15 log), so KH1 hides the Keyblade
  itself: KH1 shows it only in its battle motions (field idle / walk / run 0 / 2 / 5 carry
  none; KH1 brings it out for attacks). KH2's field / battle stance now follows KH1's battle
  mode (not KH1's slot), and while KH2's pose has the Keyblade out (`weapons_out`: battle,
  or an attack in the field) Sora's actor is kept in KH1's battle stance (`+0x374` bit
  `0x40000`, which `0x14029E050` uses to map a request to the field or battle slot) with
  the idle request replayed (`0x14029D630(actor, 0, 0.0)`); back to field when it is put
  away. Finished actions blend into idle over 14 frames (`RigAnim::queued_blend`).
- `kh1-hook` stage 17: stage 16's idle request never changed Sora's slot (stayed 0, redone
  every frame): `0x14029E050` maps a request to a slot through the motion table, and the
  stance bit only applies to actors with flag `0x4000` or no combat state, so not to Sora.
  Request bit `0x20000000` passes a slot through unmapped: the battle idle (slot 1) is now
  set directly while KH2 has the Keyblade out, the field idle (0) when it is put away.
  (The user: fetch `kh1-kh2.log` from the PC directly, never ask for it.)
- `kh1-hook` stage 18: stage 17's raw battle-idle slot took, but KH1 put the field slots
  back every frame and the Keyblade stayed hidden. The real switch (Ghidra): applying a
  motion (`0x14029DAB0`) draws the weapon for table entries of type 1 (`0x1402A0CA0(actor,
  1)`: actor flag `0x4000`, combat state `+0xC..+0xF` = ff ff ff 80) and puts it away for
  type 0 (`0x1402A10F0`: clears the flag); the flag also maps requests to their battle
  variants. The flag now follows KH2's pose (`weapons_out`). After an attack in the field,
  KH2 keeps its battle stance for 6 s (the user: combos ended in the empty-handed idle).
- `kh1-hook` stage 19 (retarget polish, the user's video: the Keyblade shows now):
  - Trouser leg snapping in falls: the thigh's roll flipped 180° within a half frame in
    falls, landings, jumps and several combo swings (`kh1-data` example `snap_check`). The
    knee's direction came from the knee's offset from the hip-ankle line, which lies along a
    raised thigh; it is now the hinge plane's (`bend`: thigh × shin), with the hips' forward
    swung along with the leg as the straight-leg fallback (`Pole::Knee`). Elbows use the same
    `bend`. Worst half-frame thigh turn: 180° → under 50°.
  - The shorts' four panel roots per leg (hip + 10 / 14 / 18 / 22) turn halfway between
    pelvis and thigh.
  - Shoes: the foot's roll is the model's up while the foot lies flat (`Pole::Up`), the shin
    only as it stands up, so soles stay flat when KH2's shins lean out.
  - The hood (root 79) turns with the chest's yaw and 30% of its lean; every bone no part
    turns now hangs as in the first frame of KH1's battle idle (slot 1, `set_rest`) instead
    of the T-pose (hood, belt chain, cloth joints, face, the Keyblade's grip and chain).
- `kh1-hook` stage 20 (the user's video of stage 19: the belt chain floated out in a loop,
  shins showed through the shorts in the battle idle, odd thumbs in the field idle, the hood
  could be lower, Sora briefly invisible, feet clip stair steps):
  - KH1 rest pose only for the hood's cloth joints (80-84) and the Keyblade (280-292); the
    belt chain (10-15) and the rest keep the bind (the battle idle's chain swings out).
  - Shorts panels follow the thigh again (the snap was the thigh roll, fixed in stage 19).
  - Hood root: chest yaw only, tipped down 0.45 rad (tip 45° below the neck).
  - Thumbs mapped (KH1 207-211 / 103-107 on KH2's thumb segment 89→92 / 181→184).
  - The skeleton pass's world matrix, once confirmed, places every later Sora at once (it
    was re-learnt per zone, placing him by his bones meanwhile: the brief invisibility).
  - Open: feet on stairs (KH1's ground IK is replaced by our pose; needs our own foot IK).
- `kh1-hook` stage 21 (the user: thumbs still odd, feet through stairs, the chain through
  the shorts, fit the jacket's lower part to KH2's motions):
  - Thumbs (103-107 / 207-211) take KH1's own pose: the battle idle's grip while KH2 has the
    Keyblade out (`takeover::GRIP`), the field idle's (slot 0) open hand otherwise
    (`Retarget::set_rest_open`, `pose_grip`). The KH2 thumb mapping is gone.
  - Belt chain root (10) turns 80% with the +x thigh; the jacket's back hem (72) 35% with the
    chest, the rest with the hips.
  - `win/feet.rs`: foot IK while KH2 has Sora on the ground. KH1's floor ray `0x1401910E0`
    (the ground-IK pass's query: start `+0`, 1.0 at `+0xC`, length `+0x14`, 1.0 at `+0x1C`; hit
    `+0x20..+0x28`, hit id i16 `+0x30`, -1 none; KH1 world y-down) under each sole (bones
    22 / 48); the body drops for a planted foot's lower floor (≤ 30), each leg bends (thigh
    17 / 43, knee 18 / 44, ankle 19 / 45) to put a planted sole on its floor or lift a raised
    one out of a step; eased (35% a frame).
- `kh1-hook` stage 22 (the user: the chain still sits oddly, the thumbs never changed, the
  jacket must mesh with KH2's motions). Measured offline before shipping (`kh1-data`
  examples `thumb_check`, `clip_check`, `kh1_follow`, `snap_check`):
  - Thumbs: KH1's own motions leave the thumbs at their bind (so no KH1 rest pose changed
    them). They are now aimed along KH2's thumb (89→92 / 181→184) with the part's rest frame
    measured on KH1's bind (`layout`'s `kh1_rest`): KH1's thumb lies exactly as KH2's
    (0° apart in every motion checked).
  - KH1 animates its cloth (belt chain, hem) by hand per motion (no blend of hips and thighs
    fits it: 20-140° residuals), so: the belt chain (10) hangs with the hips' yaw only; the
    jacket hem's nine chains (72; 248-276) turn with the hips (KH2 parents its own hem to the
    pelvis), front panels a little with their thigh; and `keep_clear` swings any cloth chain
    about its root so no joint comes nearer a thigh (hip → knee) than in the bind (≤ 14).
    Hanging and pushed cloth move at most 0.35 rad a frame (no snaps when the hips' yaw
    flips in somersaults).
- `kh1-hook` stage 23 (the user: in the battle idle the chain clips through the shorts and
  the hood through Sora):
  - The belt chain (10-15) is carried rigidly by the +x thigh (43) as in the bind
    (`ATTACHED`, `attach`): its distance to the outer shorts panel (57→59) is the bind's in
    every motion checked (a free-hanging chain fell inside the shorts when the battle
    stance spread the legs).
  - The hood chain (79: 80-84) is kept off the back (71→78) and the head (78→151) by
    `keep_clear` (up to 30), so the hunched battle stance can't tip the back into it.
- `kh1-hook` stage 24 (the user's stills: the back of the chain lies in the back of the
  shorts in every stance; the hood clips wildly in the battle idle):
  - Hood: no more hanging or pushing. Root and joints (79-84) take KH1's battle-idle relation
    to the neck base (78, on the chest part), so the hood lies on the back as in KH1 and
    moves with the chest (it can't meet the back). Tip 2-8 above the neck in KH2's stances.
  - Chain: still carried by the +x thigh; its lower joints (14, 15) also kept 4 further off
    that thigh than in the bind (constant: 13.8 / 20.1 from the thigh axis in every motion
    checked). The chain mesh's back part is presumably skinned with the hips (we don't read
    KH1's skin weights yet); if it still clips, reading the mdls mesh weights is next.
- `kh1-hook` stage 25 (the user: the hood is at 90° again; leave the chain; shoes spin round
  in the glide):
  - Hood: still carried by the chest in KH1's battle-idle relation, now folded down 0.7 rad
    about the level axis square to it (`HOOD_FOLD` in `set_rest`): tip 7 below the neck
    base, its joints 7-13 off the spine axis, constant in every motion.
  - Feet: `Pole::Foot` replaces the up rule: the foot rolls with the shin's hinge axis (the
    shin part's x, frames now computed in order, `frames`), turning to the model's up only
    while the foot is flat and its ankle low (below 24, none above 40, KH2 units). The up
    rule alone had turned the glide's pointed, sole-up feet over. `foot_check`: glide
    (205-207) shoes within 9° of KH2's feet.
- `kh1-hook` stage 26 (phase 4, combat parity, finishing):
  - Lock-on: KH1's lock state `0x142DB3F10` (non-zero while R1 holds a target) and target
    handle `0x142DB3F18` (`0x1402912F0`, the command menu, aims every command at it). KH2's
    controller locks the same enemy (`Character::set_lock`), so swings home in on it.
  - Finishers land with a KH1 finishing row whose attack data differs from the swing's
    (KH1's first 202 row shares the swing's data); every distinct attack data's first words
    are logged for picking knockback by its fields next.
- `kh1-hook` stage 27 (the user: enemies like the Large Body must repel hits from the front;
  some enemies' parts take no damage and bounce the Keyblade off):
  - KH1's guards are attack objects of priority 100 (`+0x68`; the clash `0x1402B2A30` repels
    anything meeting one), directional when type bits `0xE0000` are 1 or 3 (`0x1402B24B0`:
    Sora within the arc whose cosine is the data's `+0x64` around the owner's facing turned
    by the data's `+0x60`). `combat::guards` scans the attack pool (`0x142DB5110`, 0x40 ×
    0xC0; in use `0x800`, active `0xC`, motion-bound type 2 only while the owner plays it)
    each frame; a KH2 swing meeting one is repelled (`Character::register_wall_hit`, KH2's
    repel) and hits nothing more, with KH1's clash spark (`0x1402AC4C0(*0x142D67400, 10,
    20000, pos)`). Guards seen are logged once per owner.
  - Finishers: KH1's combo finishers are motions 203 / 206 with reaction-kind-5 data
    (0x4C58; the data's third word is the reaction kind, 3 a flinch, 5 knock-back).
  - Lock-on confirmed in the user's log (KH2 locks KH1's R1 target).
- `kh1-hook` stage 28 (Phase 5, taking hits): `win/hurt.rs` hooks KH1's hit pass `0x140299A40`.
  Before it runs for Sora under KH2, each ready hit record (queue at actor `+0x3EC`, next at
  `+0x30`, delay bits 12..15 of `+0xC`) is decided with KH2's last frame: invulnerable
  (dodge roll) → evaded, guard state / guard windows with the attacker in front (Guard Plus:
  anywhere) → KH2 guard repel + KH1 clash spark; both mark the record no-effect (`+0` bit
  0x20, as KH1's own guard does). Taken hits keep KH1's damage pass (Second Chance, MP,
  HP bar, dying) with the reaction kind cleared, then KH2 plays the reaction (3 / 4 flinch,
  5 blow) and the hurt voice `0x14028E060(actor, 0|1, 0)`, rumble `0x14028D740`, journal
  count `0x14028A4D0(0xD)` and command clear KH1 would have. Damage types 9..12 (drains,
  heals) are left to KH1. HP gone / stats flag 4: `takeover` releases Sora to KH1.
- `kh1-hook` stage 29 (Phase 6 / 7 groundwork):
  - Platform carry and push: KH1's physics step adds, before its move, the carrying
    platform's motion (`+0x200` while `+0x160` bit 4) and the frame's push (`+0x120`, zeroed
    after), and clears the touched-objects mask (`+0x90`). KH2's moves now do the same
    (`takeover::carried`, logged the first 40 times it is non-zero). Before, `+0x120` was never
    cleared while KH2 had Sora.
  - KH1's movement state (`+0x70`) is in the hand-off logs (for Phase 6/8 hand-offs).
  - Phase 7: `growth.rs` (unit tested) maps KH1's menu to KH2's abilities: save block
    `0x142DE9D60` (`*0x1428695A0`), Sora's level byte `+4`, abilities `+0x44` (48 bytes,
    bit 7 = not equipped, 0 ends the list as `0x140282A70` counts), shared abilities `+0x599`.
    KH1's own abilities map directly (High Jump levels, Glide + Superglide, Guard, Dodge Roll,
    Combo Plus, Air Combo Plus, the FM modifiers to KH2's nearest); KH2-only ones come with
    Sora's level (`LEVELS`). `win/menu.rs` reads `kh1-kh2.ini` (`abilities = test | kh1 |
    all`, default `test`, the old testing set) and logs what KH1's menu gives every time it
    changes.
- `kh1-hook` stage 30: `win/settings.rs` reads `kh1-kh2.ini` once (written with defaults when
  missing): `abilities` (test | kh1 | all), `hurt` (kh2 | kh1), `carry` (on | off),
  `ceilings` (on | off), so each feature not yet tried in game can be switched off without a
  build. Ceilings: while KH2's displacement rises, the move goes through KH1's mover in 3D
  with Sora's movement state set to 5 (falling) for the call only: `0x140297F20` builds the
  collision query's mode from the state (air mode 0x10 vs ground 4 for the player's team;
  the ground mode is what snapped rises back to the floor in stage 8). Less than 49% of the
  rise done: KH2's `hit_ceiling` (logged with the collision flags `+0x2D0`; bit 3 is what
  KH1's own jump `0x1402AD260` treats as a ceiling).
- `kh1-hook` stage 31 (Phase 6, KH1-only moves): KH1's jump runs, in its per-state input
  step (`0x1402ADA90`, the state table's third handler), checks that hand Sora to KH1's own
  air moves: `0x1402C5710` vines / climbable surfaces (state 10, collision id `+0x2DE`),
  `0x1402C68A0` ledges (state 13, from the collision's contact `+0x2F0` / `+0x300` and the
  stick), `0x1402C7E00` bars (skipped while `0x14298D3A8`), `0x1402C17B0` hang points on
  other actors (body part type 3), `0x1402BF410` swimming (state 17, water level `+0x2D4`).
  While KH2 has him falling (Air / Jump, stick held) they are tried in the same order (ini
  `grabs`); one that takes him sets `HANDOFF`: KH1 keeps him until its state is 0 / 1 and its
  motion slot a movement one again, then KH2's controller starts over standing (`FRESH`).
- `kh1-hook` stage 32 (review fixes for 28–31): KH1's fall state 3 (not 5) for KH2's rises
  and the grab checks: its collision has flag 1 (no floor snap) and mode 4 like KH1's own jump,
  its input step has the grab checks, its wall-solver extra only clears `+0x3C4`. Grabs refresh
  KH1's contacts with a standing-still move in that state first and keep the state only if a
  check took him. `HANDOFF` lasts at least 10 frames. Any release marks `FRESH` (KH2 restarts
  standing when it has him again), and the in-air exception uses `airborne` only while KH2
  has him. `hurt`: KH2 reacts only to kinds 3–5 (others stay KH1's), never restarts a blow nor
  a flinch with a flinch, plays KH2's recoil (not its guard) for attack clashes 6 / 7, and
  passes the pass's rax back. Rises take KH1's mover's height (`y = got`).
- `kh1-hook` stage 33 (Phase 8, KH1's commands stay KH1's): hook on KH1's command step
  `0x1402A5380 (actor, dir)` (called from the think step `0x1402A4090` while a command is
  pending at `0x142D54440`). Attack (type 1) still becomes KH2's attack via `exec`; types
  that only touch the party or the menu (`0x1402A1470`'s switch: 0x1D 0x1E 0x46 0x51 0x54
  0x64 → `+0x98`'s actor; 0x4B.. 0x60 → menu) pass; any other (magic is 2, spell in bits
  8..23) hands Sora to KH1 (`takeover::hand_to_kh1`, the `HANDOFF` rules). KH1 picks the
  command's version by its movement state (0 ground → `0x1402A1C20`; 2 jump → the air cast);
  in KH2's air he is first put into KH1's jump state falling from where he is, as KH1's
  ceiling bonk does (`0x1402AF7D0(actor, 2)`, `+0x3B0` / `+0x3B8` / `+0x3BC` 0, `+0x3B4` his
  y, `+0x3C0` velocity 0, `+0x3D0` 0, flag 0x10000 off). Ini `commands`.
- `kh1-hook` stage 34 (Phase 9 feel): KH1's combo row (0x14 bytes) also carries what its
  motion's start plays (`0x1402A17F0`): +2 the weapon trail (`0x1402C0160(actor, n)` asks for
  it in the combat state, `+0x38` = n | 0x40000000; `0x1402BFD70` makes it while actor
  `+0x1CC` bit 0x800000, which the attack motions' triggers set over the swing, and ends it
  when the bit clears), +3 an effect on Sora (`0x1402ABB60(*(cs+0x18), n, actor, 0x30)`),
  +0xF a voice (`0x14028E060(actor, n, 0)`). Each KH2 attack motion start plays those of the
  KH1 row of the same place in KH1's combo (200 / 201 / 202, finisher 203; air 204 / 205,
  finisher 206), and the trail bit follows KH2's hit windows (ini `swing_fx`; whether the
  trail was made is logged for the first swings).
- `kh1-hook` stage 35 (review fixes for 33–34): the trail flag is cleared whenever KH1 has
  Sora (any physics frame KH2 doesn't take, and before a KH1 command); a command hands Sora to
  KH1 only if KH1 changed his state or motion request (`+0x164`), else the in-air jump state
  is undone and KH2 keeps him; `exec` and `command` act only while KH2 has him and he is not
  handed to KH1 (`takeover::kh2_has`); a stale Attack press is dropped when KH2 starts over;
  a repel's recoil (KH2 motions 41 / 43) is not a swing start; the swing rows are cached per
  model and never empty.
- `kh1-hook` stage 36 (Phase 7, KH2's abilities in KH1's menu): KH1's ability table is
  `btltbl.bin`'s (loaded to `0x142D23740` by `0x14028D840`), entries of 12 bytes at
  `*0x142D37748` (static `0x142D26B18`), 65 of them: +0 AP, +1 kind, +2 menu order (u16; the
  menu sorts lists by it, `0x1402836C0`), +4 name handle, +8 help handle (from the remaster's
  `AbilityName.bin` / `AbilityHelp.bin`, made handles with `0x14038B080`, which registers any
  pointer's 32 MB window in the handle table `0x142EE4730`). `0x14028D760(id)` = table +
  (id - 1) * 12 is every reader (menu name `0x1402E5D40`, help `0x1402E5D10`, AP used
  `0x1402E62D0`, the message placeholder in `0x1401746D0`). `win/menu.rs` copies it into an
  80-entry table with ids 66..77 (template entry, KH2 AP, our text in KH1's codes), points
  `0x142D37748` at it (again after `0x14028D840`, hooked), and in `abilities = kh1` adds them
  to Sora's lists at their level (`growth::MENU`; shared ones on, the rest off); other modes
  take them back out. Final Mix's Combo Master (65) maps to KH2's. Commands now count as
  KH1's when the script-held flag or the motion slot changes too (spells set the flag).
- `kh1-hook` stage 37 (the user: no animation for a jump cast; the Square shortcut didn't cast;
  check moves don't repeat):
  - A command counts as KH1's when KH1 sets any motion on Sora during it (`probe`'s
    set-motion hooks count them: `SORA_MOTIONS`); the air cast (`0x14029D7F0(actor, 0x83 +
    spell)`) changed none of state / request / flag / slot at that point, so KH2 kept him and
    hid the cast's motion.
  - KH1's shortcuts are L1 + a face button: with L1 held, Circle and Square reach KH1 and are
    not KH2's jump / action (a button held past L1's release stays KH1's until let go).
  - `kh1-data/examples/combo_mix.rs` runs KH2's controller with the user's menu (Combo Plus x4,
    Air Combo Plus x2, Combo Master, the nine mapped modifiers) against near / far /
    surrounding / raised foes: no move follows itself; ground combos mix 151 / 152 / 153 with
    Slapshot, Flash Step, Slide Dash, Vicinity Break and finish with Guard Break / 155 / 154,
    air combos alternate 181 / 182 (as KH2 does) and finish with Aerial Finish.
  - A full ability list (48, KH1's most) is reported once per ability, not every second.
- `kh1-hook` stage 38 (the user: KH2 abilities have priority): when Sora's list (48) is full,
  `growth::make_room` picks a KH1 ability that is off to give up its place: first a spare copy
  of one he has two or more of (most copies first), then, in `MAKE_ROOM` order, Encounter
  Plus, EXP Zero, Cheer, Scan, Evolution, Jackpot, Lucky Strike, Treasure Magnet, Second Wind,
  Tech Boost, Leaf Bracer, MP Haste, MP Rage, Berserk, Critical Plus, Second Chance. Never an
  equipped one, ours, Guard, Dodge Roll, the specials or the modifiers. Each removal is logged
  and appended to `kh1-kh2-removed.txt` next to the game (unit-tested on the user's list).
- `kh1-hook` stage 39 (the user: keep Encounter Plus; Magnet Burst doesn't pull; where is
  Explosion?):
  - Encounter Plus is off `MAKE_ROOM`; abilities `kh1-kh2-removed.txt` lists that may no
    longer be given up are given back once a run (off), another making room if needed.
  - Magnet (ATKP reaction category 5, Magnet Burst): an enemy it strikes is caught (orbit
    radius between the attack's two knockback values, swirl its next value) and, while the
    swing lasts, drawn to the pulling hitbox's centre with the exe's suction step (toward the
    radius at up to 20 a frame plus the swirl, as `character_core::npc`) through KH1's per-frame
    push `+0x120` (its physics step adds it to the move and clears it, enemies included).
  - KH1 abilities that are KH2 moves here show their KH2 name and help in the menu
    (`growth::RENAMED`: Vortex → Flash Step, Counterattack → Counterguard, Blitz → Finishing
    Leap, Sliding Dash → Slide Dash, Hurricane Blast → Explosion, Ripple Drive → Vicinity
    Break, Stun Impact → Guard Break, Gravity Break → Aerial Finish); ids unchanged.
- `kh1-hook` stage 40 (the user: Magnet Burst's and Explosion's exact KH2 sounds and effects?):
  `win/kh2sound.rs`, the KH2 sound player (`docs/kh2-sound.md`): once KH2 Sora is loaded,
  the sounds of the effect elements the chosen motions start (`EventKind::Effect`, PAX "p_ex"
  in `obj/P_EX100.a.us`) are read from the user's KH2 install (`se/seNNN.win32.scd`, Microsoft
  ADPCM) and played through `waveOut` when KH2's clip starts the element. Ini `kh2_sounds`
  (`magnet_explosion` default: 166, 197; `all`; `off`), `kh2_volume`. KH2's visual effects
  (PAX/DPX particle systems) are not drawn yet: that needs our own particle renderer inside
  KH1's Direct3D 11 frame.
- `kh1-hook` stage 41 (the user: the KH2 sounds don't sound right; dig into the KH2 install):
  `docs/kh2-sound.md` "Sound ids are id records": `Scd::sound_id` resolves a sound id through
  the bank's id records (it was played as a stream number: wrong sounds), with
  their volume; Ogg Vorbis voice streams decode (`lewton`). `EventKind::Sound` (frame trigger
  8, weapon bank 500 = the Kingdom Key's) and `EventKind::Voice` (13 / 14, VTBL) join the
  effect elements; `kh2sound` plays all three, and KH1's swing voice stays quiet on those
  moves. Research tools: `kh1-data/examples/move_sounds.rs` (renders a move's KH2 timeline to
  WAV), `atkp_sounds.rs`; the static Linux CLI runs on the user's PC VM
  (`local/tools/kh2-combat-linux-3 extract --backup --game ...`).
- `kh1-hook` stage 42 (the user: every KH2 sound is right but the voice; keep KH1's voice):
  `kh2_voice` in `kh1-kh2.ini`, off by default: `kh2sound` loads no VTBL voices and
  `covers()` is false, so KH1's swing voice plays on Magnet Burst and Explosion with KH2's
  sounds and effect elements. `kh2_voice = on` brings KH2's battle voice back.
- `kh1-hook` stage 43 (the user: give Sora more AP to make up for the KH2 abilities): KH1's
  party stats `0x1402A2FA0` `(member, out)` fill `0x142E94BC0 + member * 0x100` from the save
  (max AP `+0xE4` = member `+9` plus the accessories' `+4`, at most 100; `0x1402E6220` is AP
  left = that minus the equipped abilities' AP, `0x1402E6120` reads it). `win/menu.rs` hooks
  it: Sora's max AP gets `growth::bonus_ap(level)`, the AP of every `growth::MENU` ability
  learned by then (27 by 50). Nothing is written to the save. `bonus_ap` in the ini.
- `kh1-hook` stage 44 (the user: KH2's real visual effects for Magnet Burst and Explosion):
  - `docs/kh2-effects.md` + `kh2_formats::dpd`: KH2's effect machine (PAX elements, 30 Hz
    effect ticks, emitters / programs / keys, MegaBirth sprites, ribbons, models, textures
    with the palette swizzle, `effect/texcommon.dpd` for VRAM addresses below 0x3000).
  - New crate `kh2-fx`: plays elements and outputs world-space triangles per batch (texture,
    GS blend, depth test, depth offset); `examples/fx_render.rs` renders them to PNGs.
  - `docs/kh1-render.md`: KH1 renders with **Direct3D 12** (D3D11 only decodes movies). Renderer
    `0x1422BB580` (device `+0x2E0`, frame buffers, constant shadows), View `0x142E9DE60`, P at
    the projection struct `+0x80`, KH1's clip formula, the world-model pass `0x1401EADD0`.
  - `win/fx12.rs`: our D3D12 drawing recorded into KH1's current list after the world pass, on
    its bound frame buffer (RTV / DSV by id, its MSAA count), own root signature (40 root
    constants + a root SRV raw buffer of RGBA8 textures, bilinear wrap in the shader) and
    PSOs per blend mode; KH1's root signature, heaps and dirty state restored with
    `0x1401087B0` / `0x140102980` (KH1 re-binds topology and vertex buffer per draw itself,
    `0x1400FBA20`). `win/kh2fx.rs`: loads `P_EX100.a.us` + texcommon from the KH2 install,
    starts elements on `EventKind::Effect`, ends motion-bound ones on a motion change, steps
    per KH2 controller frame, bones from `pose::kh2_pose`. Ini `kh2_effects`, `fx_test`.
- `kh1-hook` stage 45 (the user: the game crashed at Explosion's effect): KH1's current list is
  usually a **bundle** (`0x1400FE6E0` creates its allocator and list with type 1;
  `0x1400F7290` closes it and runs it with `ExecuteBundle` in the direct list at command
  buffer `+8`). Our `OMSetRenderTargets` is not allowed in a bundle, so its `Close` failed
  (`E_INVALIDARG`) and KH1's `ExecuteBundle` of the broken bundle crashed. `fx12` now checks
  the list type (`GetType`) and sets targets only in a direct list (a bundle inherits them),
  then hands KH1 a fresh list with its own mid-scene sequence (`0x1400F7300(R, 0)`,
  `0x140109360(R, 1, 0)`, as in `0x1400FBBA0`) instead of `0x1401087B0` + `0x140102980`.
- `kh1-hook` stage 46 (the user: casting Thunder crashed the game): the hit pass `0x140299A40`
  takes **two** arguments, `(actor, out vector)` (rdx, kept in r13 and passed to the
  reactions as r8; the knock-back reaction `0x14029AD30` writes the actor's position there).
  `win/hurt.rs` hooked it as `(actor)` and called KH1's with a garbage rdx; harmless while
  KH2 had Sora (KH1's reaction kind cleared), it crashed when a hit landed while KH1 had him
  (a spell cast). The hook now takes and passes both.
- `kh1-hook` stage 47 (the user's ability levels): High Jump (one) = High Jump MAX; Glide =
  Glide LV3, Superglide = Glide MAX; Quick Run and Aerial Dodge LV3 when learned (their level
  growth dropped; the dodge roll keeps its own). Sora keeps one High Jump / Glide / Superglide
  (`growth::one_only`, an equipped copy kept): spares are taken out of the shared list in
  `abilities = kh1` and noted in `kh1-kh2-removed.txt` (without "(id", so they are not given
  back).
- `kh1-hook` stage 48 (the user): High Jump is High Jump LV3 (not MAX).
- `kh1-hook` stage 49 (hook audit, after stage 46's crash): for every hooked function, the
  argument registers it reads before writing (rcx / rdx / r8 / r9, xmm0..3) and whether its
  callers use rax / xmm0 after the call (a Ghidra `CallUse.java` over every call site), and the
  same for every KH1 function we call. Found: `0x14029D7F0` and `0x14029D8A0` (set motion)
  take a float third argument in XMM2 (stored at actor `+0x184`) and their result is used by
  callers; `0x140392710` (skeleton pass with ground IK) takes a float third argument. The hooks
  declared those as integers, so KH1 got whatever XMM2 held: now `f32`. Everything else matches
  (`0x1402A4760` 3 ints, `0x1402A5380` 2, `0x140299A40` 2 (46), `0x1401EADD0` /
  `0x14028D840` none, `0x1402A2FA0` 2, `0x140391E20` (int, int, float, int), `0x14029CF10`
  (int, float), `0x140112C70` 3, `0x14029D630` (int, int, float), `0x1402BD180` 2 → used
  result, `0x140298230` 1).
- `kh1-hook` stage 50 (the user: softlocked in the Dive to the Heart's attack tutorial; the
  Dream Sword held backwards; Sora moving while KH1 holds him still; without Combo Master one
  swing only):
  - KH1's attack step (`0x1402A4760`) counts each attack in the save's tallies
    (`0x14028A4D0(0x13)`: `+0x848 + id * 2`, capped 9999); event scripts read the tallies
    (`0x14028A4B0`, an event opcode at `0x1401ADF??`). KH2's swings now count too.
  - KH1's player control step `0x1402A4090 (actor, move vector)` (from the think step
    `0x1402A1170`, skipped when actor `+0x444` bit 1 is set) reads the pad and starts
    commands only when `0x1402956E0` says he can act (flag 4, `+0x88` bit 0x20, `+0x444`
    bit 1). Hooked: KH2 gets no input on a frame it didn't run with him able to act.
  - The weapon's grip bone 280: when KH1's own grip (relative to its parent, read from KH1's
    pose while it shows slots 0-7) is turned more than 1 rad from the battle idle's, it is
    used (`Retarget::set_grip`): the Dream Sword is held another way than the Keyblade.
  - `Abilities::one_swing_on_miss` (character-core, ours, set in `abilities = kh1`): without
    Combo Master a swing that hasn't connected doesn't chain.
- `kh1-hook` stage 51 (the user: at the start of some cutscenes Sora slides and snaps back;
  the Dream Shield is held backwards): held still on the ground (KH1's control step skipped
  or him unable to act) for 2 frames, KH1 has him (KH2's run slide or lunge no longer moves
  him while the event sets up), KH2 starts over afterwards. Logged once per weapon: Sora's
  weapon (save `+0x36`) and KH1's grip of it against the Keyblade's rest grip (for the
  shield's fix).
- `kh1-hook` stage 52 (the user: the Dream Shield's front faces Sora): ini `flip_weapons`
  (KH1 item ids, default 0x53, taken to be the Dream Shield: KH1's weapon items run Kingdom
  Key 0x51, Dream Sword, Shield, Rod; unconfirmed, the weapon line in the log gives the id):
  the grip bone's subtree is turned half round about the forearm at the hand
  (`pose::turn_weapon`).
- `kh1-hook` stage 53 (the user's video: with the Dream Shield, KH1's idle and attacks
  showed, a choppy cut out of a run, no attack while walking): the shield has its own
  moveset, whose motions our slot table (P_EX100's signatures) doesn't know, and an unknown
  slot was taken for KH1's own action (KH1 had him). Now only a known slot past 7 is; KH1's
  move hand-back takes an unknown motion as standing when KH1's last request was a slot up to
  7 (`kh1_moving`). Confirmed in the log: the Dream Shield is item 0x53 (`flip_weapons`'s
  default). The live-grip switch of stage 50 is gone (KH1's grip moves with its motions; it
  flapped between the two every few frames); its log line stays.
- `kh1-hook` stage 54 (the user: lock-on on Darkside's hand still sent Sora at his body;
  Wakka's ball can't be hit back; the Dream Shield still faced Sora; KH1's own moves (ladder)
  snapped in; items from chests were Archipelago items):
  - `docs/kh1-lockon.md`: KH1 locks a (handle, lock point) pair: point index at
    `0x142DB3F1C`; `0x1402AE330 (actor, point, out vec4) -> radius` is the point's world
    position (lock points: collision record `+7` count / `+0x1C` list, 0x14 each, mask actor
    `+0x3AC`). KH2's target for the locked actor is now that point.
  - `docs/kh1-reflect.md`: shots like Wakka's ball are free-standing attack objects of the
    pool (`+0x30` bit 0x400); a swing turns them back through the attack-vs-attack meeting
    `0x1402B2890 (entry, entry)` (`0x1402B2A30` both ways: team swapped, Sora the owner, flag
    0x1000000). `combat::shots` / `meet_shot`: KH2's swept swing against them, once a swing,
    with a KH1 row's attack object as the swing side.
  - The Dream Shield (`flip_weapons`): held with KH1's own wrist (the grip's parent bone
    relative to its parent, from KH1's pose each frame it poses him), carrying the fingers and
    the weapon; KH2's arm swings as before. (The half-turn of stage 52 is gone.)
  - KH1's own animation blends in from the last pose we drew over 0.2 s (`KH1_BLEND`).
  - The user's KH1 `Image/en/kh1_first` was a randomizer-patched package (Feb 2025: 26 files,
    Destiny Islands / Dive / Disney Castle `.ard`s, `UK_treasure.ev`, ...): restored from
    `BackupImage/`, the patched pair kept as `*.ap-patched`.
- `kh1-hook` stage 55 (Wakka's ball still not turned back; nothing in the log): the swing now
  meets every live enemy attack of the pool but guards (KH1's pass meets every pair; the
  meeting decides), and each one near Sora is logged once by slot and kind ("enemy attack
  near Sora: ...") to tell what the ball is.
- `kh1-hook` stage 56 (diagnostics for the user's T-pose arms, seen on the ladder and at
  Darkside): logs when KH1's own pass leaves both hands where the bind has them relative to
  the chest (with the pass, slot, the ground-IK pass's float argument (a blend weight, used
  by `0x140392710`'s lerps) and mode), and when the pose we draw does.
- `kh1-hook` stage 57 (the user's ladder video: "T-pose" arms dropping off the ladder): not a
  T-pose but KH2's battle fall (motion 4, arms flung up and out: hands at head height ±55
  apart in `retarget_check`) drawn for KH1's own fall (slot 10) while KH1 had him. KH1's own
  jump / fall / landing (slots 8-13) now show KH1's own animation while KH1 has him (KH2's
  jumps are KH2's as before). The T-pose detector of stage 56 is gone (its chest bone was
  three parents up from the hand, still in the arm: it fired in every idle).
- `kh1-hook` stage 58 (the user: walking into a ladder, the walk froze before the climb):
  `pose` kept drawing KH2's last pose for 0.15 s after KH1 took him (`takeover::POSE`'s
  freshness), then blended from it: a frozen walk for a third of a second. Taking him to KH1
  now drops that pose (`takeover::drop_pose`), and the 0.2 s blend into KH1's own animation is
  eased out (KH1's motion shows at once, our last pose fades).
- `kh1-hook` stage 59 (the user: can't land on boxes; Sora hung over a box in KH2's fall,
  arms out): the map floor point `+0x320` doesn't know objects, and KH2's falls set his height
  against it alone. Falls now go through KH1's mover in its fall state too (as rises did), which
  stops him on objects; a fall stopped short (less than half of it happened) lands him there
  (`Driver::on_top`), and on the ground he stays on top while the ground mover keeps him at
  that height (else the usual floor step-down / walk-off).

### 2026-10-16: Stage 60 (fewer clinks, wall clinks, box tops)
- **Enemy attacks a swing meets** follow KH1's own pass (`0x1402B2610` / `0x1402B1A50`): motion-tied
  attacks (activity 2) only inside their owner's hit window, `0xFFF` latching skipped, word bit
  0x400 and spent 0x80 attacks left out, the pairing table `0x1405299B8` checked both ways. The
  user: endless clinks against the enemies' resting attack shapes.
- **Walls:** each live KH2 swing casts KH1's background ray (`0x140191520`) from last frame's
  attack-shape centres to this frame's; a map face whose normal is mostly level (a wall) repels
  the swing (`register_wall_hit`, KH1's clang). Floors, ceilings and objects don't.
- **Box tops:** KH1's object-top finder `0x1402B3950 (actor, move) -> object` (types 2 / 3
  collision shapes of standable objects) is run in KH1's air state 2 (it skips state 3's class)
  from where a fall started, and each grounded frame a step down; he lands / stands on the top
  and `+0x398` is set (`0x140295C60`). The mover in our states never found them.


### 2026-10-16: Stage 61 (aim only at KH1's target)
- KH2's controller gets only KH1's current target (R1 lock, state 3, or the small cursor's soft
  target, states 1/2/4) in its target list, so swings neither turn toward nor home in on any
  other enemy or lock point; with none, a swing goes where the stick points. Ini `auto_aim = on`
  brings back KH2's own nearest-in-reach pick. The user: the automatic aim was too heavy.

### 2026-10-16: Stage 62 (wall clinks off by default)
- The user changed his mind: swings pass through walls again. The wall ray stays behind ini
  `wall_clink = on` (default off).

### 2026-10-16: Stage 63 (Quick Run LV2 later, both with AP, "New Ability" prompts)
- Quick Run: LV2, learned at level 12 (was 7); Quick Run (2 AP) and Aerial Dodge (3 AP) are in
  Sora's own list now, off until equipped, and their AP joins `bonus_ap`. Saves from earlier
  builds have them moved out of the shared list (one that was on stays on).
- Learning a KH2 ability shows a box in KH1's level-up prompt style ("New Ability", its name,
  its AP), through KH1's prompt boxes `0x14283BD90` / text `0x142DC3068` the way the
  Archipelago KH1 client shows items (gaithern's KH-1FM-AP-LUA, MIT, `show_prompt`); queued
  until Sora's box is free (a level-up's own box goes first).

### 2026-10-16: Stage 64 (walk out of the dodge roll)
- From the roll's fall-check event (frame 34 for Sora) the stick ends the roll and Sora walks /
  runs at once, as out of a landing; before, only a button did, so he stood still until the
  motion ended (the user: too much of a freeze after the roll).

### 2026-10-16: Stage 65 (New Ability box: the name only)
- The "New Ability" box shows the ability's name alone (one line), the user's choice.

### 2026-10-16: Stage 66 (KH2's field camera, Camera Style in KH1's Config menu)
- Research: `docs/kh1-camera.md` (KH1's field camera module, its mode table and state) and
  `docs/kh1-config-menu.md` (the Config menu's window, tables, text lookup, storage).
- **`win/camera.rs`:** KH1's camera mode table (`0x140507AD0`) gets ours for mode 1 (follow)
  and mode 3 (lock-on). Each frame `kh2_camera::Camera` (KH2's free follow / lock-on framing,
  wall pull-in with KH1's camera ray `0x140191640`) writes KH1's internal eye / target, with
  KH1's yaw / pitch / distance kept in step and its own timers zeroed; KH1 still blends to and
  from its other cameras, runs its climbing / ledge cameras (`0x1401E3320`), events and area
  cameras, and builds the View. Right stick: x turns (KH1's left/right option's sign), y zooms;
  R3 resets; Select is KH1's first person; KH1's Auto / Manual option is KH2's Auto. KH1 world
  ↔ camera: y flipped and divided by `camera_scale` (0.9).
- **`win/config.rs`:** a ninth Config row "Camera Style: KH1 / KH2". The window definition's
  row counts go to 9; its update is wrapped (our row's left / right / confirm switch the
  camera and are hidden from KH1's update; the row is drawn with KH1's panel calls; rows 25
  apart during the call so nine fit); the help line's id is swapped for ours on our row; the
  sysmsg lookup `0x1402CE7C0` is replaced (our ids `0x7F000001..4`). The choice is saved in the
  ini (`camera`), not the save.
- Untested in game: the row's layout against the help box, the text codes in sysmsg, the
  camera's turn direction and scale.

### 2026-10-16: Stage 67 (KH2 camera: right stick x the right way round)
- The right stick's x is negated for `kh2_camera` (the user: x was inverted).

### 2026-10-16: Stage 68 (KH2 camera at KH2's own distance)
- `camera_scale` defaults to 1.0 (KH2's distances and heights as they are), the user: a bit
  further back, as in KH2.

### 2026-10-16: Stage 69 (KH2 camera lowered)
- `camera_lower` (default 70): KH2's camera, look point and eye together, that much lower, so
  KH1's smaller Sora sits near the middle of the screen (the user). Same angle as KH2's.

### 2026-10-16: Stage 70 (KH1's camera in cutscenes)
- KH2's camera gives way to KH1's own follow / lock-on camera while an event runs (talks with
  text boxes included), an area script walks Sora, or KH1 holds him still
  (`takeover::kh1_scene`); KH1's carries on from where KH2's was. After a gap KH2's starts
  again from KH1's view (yaw and distance), not from where it left off.

### 2026-10-16: Stage 71 (the Jungle Slider is KH1's)
- KH1 movement state 30 (the Jungle Slider) hands Sora to KH1 for as long as it lasts (the
  user: he walked along the vines instead of sliding); KH2's camera gives way there too.
  `KH1_MODES` in `takeover.rs` lists such states.

### 2026-10-16: Stage 72 (no hanging on the sides of crates and tents)
- A fall the air mover stopped short counts as a landing only when KH1's collision has him on
  ground (`+0x2D0` bit 0 clear); a wall beside him stopped it too, and he hung on the sides of
  crates and the camp's tent, flickering between landing and falling (the user's video).
  Otherwise the fall keeps KH2's own height. On the ground he stays on such a landing while
  KH1's collision keeps him on ground there.

### 2026-10-16: Stage 73 (all of the Jungle Slider is KH1's)
- The slider also runs in KH1 movement state 31 (the user: its jump and attack didn't work, KH2
  had Sora there): every state from 30 up is KH1's (`kh1_mode`).

### 2026-10-16: Stage 74 (catching a ladder on the way down)
- The grab probe's air move presses 4 units along the stick instead of standing still: KH1's
  climb check (`0x1402C5D30`) needs the wall face the move touched (`+0x2DE`), which a
  standing-still move cleared, so a ladder jumped into was caught only at its foot (the user).
  Without a grab the position is put back.

### 2026-10-16: Stage 75 (a roll right after KH1's own move is KH2's)
- While KH1 has Sora back on the ground after one of its own air moves (`HANDOFF`, state 0/1),
  KH1 doesn't see Square, and a Square press ends the handoff at once with KH2 taking it as a
  fresh press (KH2's roll). Before, KH1 started its own roll there (slot 117) and KH2 took over
  mid-way: he slid backwards (the user, jumping off a pole and rolling).

### 2026-10-16: Stage 76 (KH2 camera on entering a room)
- A new KH2 camera starts at KH2's default distance, behind Sora unless KH1's camera was
  looking at him from a usual distance (then its yaw); and one is started when Sora is far from
  the look point (a new room, a warp). Before, it took KH1's left-over pair's distance, and
  KH2's closest distance puts the eye 150 below the look point: on the ground looking up (the
  user, every room entry).

### 2026-10-16: Stage 77 (no clashing with enemies' melee attacks)
- A KH2 swing meets only free-standing enemy shots (`+0x30` bit 0x400: thrown and fired
  things, Wakka's ball among them, whose log entries showed `+0x30 = 0x541`). Enemies' melee
  attack shapes no longer clash with it (the user, again: far too many clinks); the swing hits
  their bodies as in KH2.

### 2026-10-16: Stage 78 (KH2 camera parity)
- KH2's field of view: 69.9° vertical (1.5 rad across 4:3), set through KH1's FOV setter as 76.1
  "PS2 degrees" each frame KH2's camera runs (KH1's 53 is 48° vertical); KH1's default comes
  back when KH1's camera runs.
- Framed as in KH2: `camera_scale` 0.89 (KH1 / KH2 Sora's hips) and `camera_lower` 0.
- Sticks: the pad's raw ones with a 15% dead zone (KH1's processed ones drop the inner 47%);
  KH1's processed right stick when the raw one is idle (mouse).

### 2026-10-16: Stage 79 (New Ability box after the level-up's own)
- The "New Ability" box waits until KH1's prompt box has been free for 4 s: a level-up's own
  box comes a moment after the level changes and replaced ours (Aerial Dodge's was logged as
  shown, the user never saw it).

### 2026-10-16: Stage 80 (New Ability box beside the level-up's)
- KH1's prompt column holds up to 5 boxes per party member (`0xBA0` apart, count 16 bytes before
  the first). For 3 s after Sora learns an ability its box waits for KH1's own (the level-up's)
  and is added as the next box in Sora's column, with its own text buffers; with none by then it
  shows alone once the column is free (the user: show both at once).

### 2026-10-16: Stage 81 (Aerial Dodge after High Jump; grabs as KH1's jump tries them)
- Aerial Dodge is learned at level 13 only once Sora has High Jump (the user); without it, it is
  taken back out of his list, and its AP isn't in `bonus_ap`.
- Air grabs (ladders, vines, poles, ledges) are tried as KH1's jump step does: also near the top
  of the jump (rising under 2 a frame) and with the stick neutral (then no probe move: the
  frame's own air move left the contacts). The user: hit or miss from a jump.

### 2026-10-16: Stage 82 (bosses give KH2 moves)
- `growth::BOSS_GIFTS` (the user's list): Sabor Upper Slash, Clayton Aerial Sweep, Jafar Aerial
  Finish (KH1's Gravity Break), Pot Centipede Guard Break (Stun Impact), Dragon Maleficent
  Explosion (Hurricane Blast), Captain Hook Magnet Burst, Cerberus Aerial Dive. Learned once the
  world's story progress byte (save `+0x1504`, `0x142DEB264`, 12 worlds) reaches the value it has
  after the boss (thresholds as the Archipelago KH1 client reads them), unless Sora already has
  it; off until equipped, with a "New Ability" box. Upper Slash, Aerial Dive and Magnet Burst
  are no longer learned by level (`MenuAbility::level` = `BOSS`); their AP joins `bonus_ap` once
  the boss is beaten.

### 2026-10-16: Stage 83 (Aerial Sweep at level 28)
- `growth::LEVEL_GIFTS`: KH1's Aerial Sweep also at level 28 (the user; KH1's Dream Shield gives
  it at 84), unless Clayton already gave it. Guard Break stays at KH1's level 18 (the user).

### 2026-10-16: Stage 84 (Upper Slash works)
- Upper Slash's PTYA row (12, kind 0, motion 161) was never reachable (kind 0 rows aren't picked
  by `select`). As KH2's help has it: the action button (Square) in a ground combo's combo or
  finisher window starts it (forced row), when equipped. Retaliating Slash also takes the
  action button now (KH2's), besides attack. Menu help texts for both rewritten to say so.

### 2026-10-16: Stage 85 (Aerial Recovery has something to recover from)
- KH1's knock-back hits (kind 4) are KH2 blows now, as its launches (5) were; every 4 was a
  flinch, so Sora was hardly ever knocked into the air (the user: Aerial Recovery never came
  out). Aerial Recovery (jump) also works from an air flinch (hit while airborne).

### 2026-10-16: Stage 86 (KH2's Square moves)
- KH2's Square combo moves are kind-0 PTYA rows, never picked by `select`. The action button in
  a combo or finisher window now tries, in order, Finishing Leap (row 37, ability 95: the
  combo's end), Upper Slash (12, 18) on the ground and Horizontal Slash (38, 99) in the air,
  as KH2's help texts describe them (the user: Horizontal Slash didn't work). Menu help texts
  updated.

### 2026-10-16: Stage 87 (KH2 launches launch)
- Research: `docs/kh1-launch.md` (KH1's hit record, an enemy's launch reaction `0x1402B6390`,
  its arc fields). KH1 has no per-hit launch speed or angle.
- A KH2 hit whose ATKP reaction is a launch (category 4) forces KH1's launch (record kind 5, in
  `scale_record`) and, once KH1's launch reaction has run (hooked) and the enemy did launch
  (motion 0x52 / 0x5C, state 2), sets KH2's apex height and rise time (`+0x3BC` / `+0x3B8`, from
  the ATKP knockback values flown as KH2's blow flies) and horizontal speed (`+0x3C0`). Upper
  Slash goes up (the user). Enemies without a launch motion (big ones, most bosses) don't.

### 2026-10-16: Stage 88 (no attacks mid-launch; KH2 camera through summons)
- A forced combo row (Counterguard, Retaliating Slash, the Square moves) that can't be used is no
  attack: it fell back to the ordinary combo, so Sora attacked while knocked into the air (the
  user) and Square mid-combo could start a plain swing.
- KH1 holding Sora still counts as a scene for the camera only out of battle (the user: KH2's
  camera should stay through a summon's attack, Simba's charge).
- Launch logs: each KH2 launch asked for, and KH1 launch reactions that weren't ours.

### 2026-10-16: Stage 89 (a tapped Quick Run goes less far)
- Letting go of the action button before Quick Run's time is up stops it with a stronger brake,
  the earlier the stronger (0.7 per tick for a tap, KH2's stop brake when held to the end): at
  LV2 a tap went ~740 units against ~950 held, as KH2's end slide is most of it (the user).

### 2026-10-16: Stage 90 (KH2 camera through Simba's charge)
- Only the Jungle Slider's movement states (30, 31) give the camera to KH1; a summon's (38
  Simba's charge, 40 the call) keep KH2's (the user). KH1 still has Sora in all states 30+.

### 2026-10-16: Stage 91 (Quick Run grows with Sora; Aerial Recovery as the exe)
- The tap brake (stage 89) is gone (the user). Quick Run grows with Sora instead
  (`growth::QUICK_RUN_LEVELS`): LV1 at 12, LV2 at 20, LV3 at 32, MAX at 45. Its menu name is a
  buffer rewritten in place ("Quick Run LV2" ... "Quick Run MAX"), and each growth shows an
  "Ability Up" box.
- Aerial Recovery per `docs/aerial-recovery.md` (exe `0x1403D6030`): launches only (not air
  flinches), from the launch's 16th frame, jump: motion 40, hop 100, 10/frame along the stick
  (straight up with it neutral), invulnerable to frame 16, control back at frame 24, no Aerial
  Dodge after. Retaliating Slash: Square only, same frame.

### 2026-10-16: Stage 92 (Dodge Roll shows its level)
- KH1's Dodge Roll (22) gets a name buffer like Quick Run's ("Dodge Roll LV1" .. "MAX", its
  levels as before: 19 / 37 / 60), written into our ability table's entry, and an "Ability Up"
  box each time it grows (the user).

### 2026-10-16: Stage 94 (High Jump and Aerial Dodge grow with Sora)
- High Jump (KH1's 1, shared) and Aerial Dodge (our 67) level up like Quick Run and Dodge Roll
  (the user): High Jump LV2 / LV3 / MAX at 24 / 38 / 52, Aerial Dodge at 25 / 36 / 50
  (`HIGH_JUMP_LEVELS`, `AERIAL_DODGE_LEVELS`), LV1 when Sora gets them. Each gets a name buffer
  ("High Jump LV2", entry 1's handle like Dodge Roll's; Aerial Dodge's in the TEXT init like
  Quick Run's) and an "Ability Up" box when it grows; learning Aerial Dodge shows its level.

### 2026-10-16: Stage 95 (no leftover launches)
- The user's log: Upper Slash's launch reaches Shadows (up 265 in 26 frames), but launches
  asked on Cerberus (which KH1 never launches) stayed queued under KH1's reused hit record and
  one landed on a Shadow later. A KH2 launch now waits only 250 ms (`LAUNCH_FRESH_MS`) for
  KH1's launch reaction.

### 2026-10-16: Stage 96 (ground-to-air rows)
- The user: Aerial Sweep and Aerial Dive (ground-to-air) were hard to trigger. KH2's combo
  selector (`0x140401BB0`) measures facing from the direction to the primary target, not the
  player's facing; kind-1 rows have a facing window of 0.99996..1, so with Sora's own facing
  they (Aerial Sweep 33, Aerial Dive 48, the jump at an airborne foe 0, Slide Dash, Flash
  Step, the long approach 153, Aerial Spiral) almost never passed. `Controller::try_attack`
  now aims at the primary target for the selection. The height window's sign was checked too
  (`0x140401AD0`: target − player, y-down; the doc had it reversed, the code was right).
- Aerial Dive's help: from the ground, at a far airborne foe.

### 2026-10-16: Stage 97 (four shortcuts)
- The user: four shortcut slots like KH2, KH1's shortcut menu extended so it fits in.
  `win/shortcuts.rs`, `docs/kh1-shortcuts.md`. The fourth is L1 + the cancel button (Circle;
  Cross with the button swap), kept at save `+0x847` (spell + 1; old saves start empty), set
  in Customize (a fourth row, its label KH1's third with the cancel icon), filled with a newly
  learned spell when KH1's three are full. In battle the shortcut menu moves up a row, its
  panel is stretched by a row (cut at the middle, a one-texel band between), and the fourth
  row gets the cancel icon. `shortcuts = 3` in the ini turns it off.
- The text lookup (`config.rs`) is in even with `config_row = off`; an id past KH1's table reads
  as empty.
- The previous run's log is kept as `kh1-kh2.prev.log`.

### 2026-10-16: Stage 98 (the shortcut menu looks like KH1's)
- The user's screenshots: the battle panel cut in the middle looked wrong and the title
  ("SHORTCUT", the second panel: 8 px tall) moved into the rows (its quad is in GS
  coordinates, not the menu's, so the "lower rows" test always passed). The panel quad (header
  and three rows, 86 px, UV 1:1) is now drawn whole and its last 22 px drawn again a row lower;
  the title stays. In Customize the fourth label showed garbage: KH1's label 0xD0 is
  `0D 16 00 ...` (an x offset whose operand has a 0 byte), so it was cut short; the label is
  now read code by code (`0x1402CFEA0`'s operand sizes) and the confirm icon (`0F 41`) becomes
  cancel's (`0F 42`).

### 2026-10-16: Stage 99 (the gradient over four rows)
- The user: the shortcut panel's gradient should go all the way down. The panel is drawn as its
  header and four 22 px rows cut from KH1's three; row k shows texture row 2k/3, the two
  nearest rows blended (the second drawn over with vertex alpha), so the shading spreads over
  four rows and the shapes stay KH1's. The Customize label came out `0d 16 00 0f 42` (right).

### 2026-10-16: Stage 100 (lock-on camera steady)
- The user's video (KH2 camera, locked on in the Cave of Wonders): the camera jumped about.
  The "Sora is far from where it was looking" restart compared the look point with his feet;
  locked on, the look point is 30% of the way to the target, so a far target restarted the
  camera (from KH1's eye) again and again. It compares the eye now (1200). A lock-on target
  with no point this frame no longer hands the frame to KH1's lock-on camera: ours follows.
  Restarts and hand-overs are logged with their reason.

### 2026-10-16: Stage 101 (targets as KH2's target shapes)
- The user: Aerial Dive still barely came out. KH2's combo table measures a target by its
  target shape (`0x1403C8EE0`: the shape's point raised by its position height, and its own
  size), and Aerial Sweep / Aerial Dive / the jump at an airborne foe (flag 0xA: the window's
  low end shrinks by the target's height) want that point at least 110 + its size above Sora.
  We gave targets their feet and whole height (a flyer needed its feet ~200 up). Every target
  is now KH1's lock point 0 (its centre, the lock radius as its size), the locked one too (it
  was the sphere's bottom and twice the radius); a target without lock points keeps its feet
  and height. Magnet Burst's pull still aims at the body's middle (`Body::tall`).

### 2026-10-16: Stage 102 (the magic carpet)
- The user's video: the magic carpet out of the Cave of Wonders (after Genie Jafar) didn't
  move. The log: KH1's control step for Sora came and went every few frames ("KH1 holds Sora
  still" / "lets Sora act again" over and over), and KH2 took him on the frames between, so
  KH1's own physics (which the ride needs) didn't run. KH1's control coming and going 6 times
  in 3 s is now taken for a ride: KH1 has him while it lasts and 3 s after. The control step
  is read once per frame at the start of `takeover` (`FROZEN_NOW`), before any early return.
  The hold / act lines now carry his state, slot, flags, motion and the object he stands on.

### 2026-10-16: Stage 103 (Aerial Finish holds)
- The user: enemies were pushed out of the air finisher. Aerial Finish's spinning hits
  (ATKP 915 / 916 / 917: reaction 9, up 10 / 5 / 12, sideways -1 / -1 / -10) are blows that
  hold the foe up beside Sora; landed with KH1's air finisher row they sent it ~250 away on the
  first hit (the log: 915 at z -1836, 916 at -2089) and the rest missed. A blow takes KH1's
  finisher row (and a KH2 launch) only when it throws: sideways >= 10 or up >= 30 (or the
  default blow); the others land with the plain row. The last hit (927: 3 / 45) still throws.

### 2026-10-16: Stage 104 (invulnerable frames)
- The user: Aerial Recovery, Aerial Dodge and Quick Run should have invulnerable frames.
  Aerial Recovery already had KH2's (to frame 16). Aerial Dodge (none in KH2) gets 10 / 12 /
  14 / 16 by level (`AIR_DODGE_STAR` + 2 per level). Quick Run keeps KH2's fmab star times
  (0 / 8 / 12 / 16 by level) with at least 6 (`QUICK_RUN_STAR_MIN`, for LV1).

### 2026-10-16: Stage 105 (Riku's Aerial Dodge, later growth, holding hits stay put)
- The user: Aerial Dodge for beating Riku the first time (Hollow Bastion progress 0x32, its
  reward's threshold in gaithern's KH-1FM-AP-LUA, MIT; 0x28 during the fight), no longer a
  level or High Jump's (the High Jump gate and its take-back are gone; the user's Archipelago
  save has no High Jump). Its New Ability box shows its level. High Jump grows at 30 / 44 / 58,
  Aerial Dodge at 40 / 52 / 64.
- Still sliding out of Aerial Finish: KH1's flinch on an airborne foe (kind 3, `0x1402B69E0`)
  knocks it along the record's direction each spin. A holding blow's record direction is now
  zeroed (in `scale_record` and again after the contact), so it flinches in place.

### 2026-10-16: Stage 106 (no Sonic Blade in the air)
- The user: Sora could use Sonic Blade mid-jump. KH1's command check (`0x1402912F0`) allows
  only magic (kind 2), kind 99 and kind 0x47 in the air, judging the air by his flags
  (`0x1402A0D30`: `+0` bit 4, `+0x88` bit 0x20, `+0x444` bit 1), which stay the ground's while
  KH2 has him. `combat::command` now refuses any other command while KH2 has him in the air,
  with KH1's "can't" sound (`0x140177F60 (4)`). Magic stays allowed (jump casts).

### 2026-10-16: Stage 107 (Aerial Dodge taken back until Riku)
- The user's save still had Aerial Dodge (from its old level). It is taken out of Sora's list
  while Riku (Hollow Bastion, first time) isn't beaten, and given again (with its box) once he
  is.

### 2026-10-16: Stage 108 (KH2's sounds for every ability move)
- The user: the exact KH2 sounds for the KH2 moves, as Explosion's and Magnet Burst's (no
  visuals). `kh2sound::DEFAULT_MOTIONS` is now every ability motion of Sora's PTYA (161..172,
  191..197: Upper Slash, Slapshot, Dodge Slash, Slide Dash, Guard Break, Explosion, Finishing
  Leap, Flash Step, Vicinity Break, Counterguard, Retaliating Slash, Aerial Sweep, Aerial
  Spiral, Horizontal Slash, Aerial Finish, Aerial Dive, Magnet Burst); the plain combo keeps
  KH1's. `kh2_effects` keeps its own two (`kh2fx::DEFAULT_MOTIONS`). Any `kh2_sounds` word
  but `all` / `off` (the old `magnet_explosion` too) means the new default.

### 2026-10-16: Stage 109 (no Sonic Blade in the air, again)
- The user: Sonic Blade still came out mid-jump; the log had it as a ground command (command
  0x4, not "in the air"). `kh2_airborne` read the driver with `try_lock`, and KH1's command
  step can run while it is held (a failed lock read as "on the ground"): the takeover now keeps
  `KH2_AIR` each frame. The refusal also runs while KH1 has him in its fall state 2 (after one
  of our air casts, whose state we set without KH1's airborne flags), before the hand-off
  test, and every command logs whether KH2 had him in the air.

### 2026-10-16: Stage 110 (the ride test only while KH2 has him)
- The user: Sora stuck in KH1's animations. Stage 102's ride test counted KH1's control coming
  and going on every frame, KH1's own included (a room entry's script holds, then KH1's own
  frames, which flicker too), and each count kept the ride going: KH1 had him for good. It now
  counts only on frames KH2 has him, needs 10 changes in 3 s, and gives KH1 him for 5 s
  without extending.

### 2026-10-16: Stage 111 (the magic carpet, for certain)
- The user can't easily get back to the carpet and it softlocks if broken. `0x1402957F0` (the
  player actor's setup) puts Sora in movement state 29 (0x1D) in Agrabah (world 8) room 0x14
  while its event index is under 0xD: the carpet out of the Cave of Wonders. KH1's own modes
  were 30 and up; now 29, 24 and 25 too (24 is Atlantica's water, set when `0x14298D3A0`, world
  9 or End of the World room 0x15, `0x140288920`; 24 / 25 are the states where jump means up,
  `0x140291660`'s mask 0x203000100). KH1's camera for 24 / 25 / 29 / 30 / 31. (Stage 110's
  flicker test stays as a fallback.)

### 2026-10-16: Stage 112 (KH2-named KH1 abilities earlier)
- The user: Vicinity Break at 69 (Dream Shield) is absurd, Slide Dash at about 30. `LEVEL_GIFTS`
  now also gives Counterguard (Counterattack) at 22, Slide Dash (Sliding Dash) at 30, Vicinity
  Break (Ripple Drive) at 36, Flash Step (Vortex) at 42 and Finishing Leap (Blitz) at 48, with
  Aerial Sweep at 28 as before (off, with a New Ability box, if Sora doesn't have it yet).

### 2026-10-16: Stage 113 (Aerial Finish holds its foe)
- The user: foes still slipped out of Aerial Finish. The log: whole spins land sometimes, but
  often the foe is gone after 915 / 916 (falling while Sora spins up). KH2's reaction-9 blows
  lift and pull the foe toward Sora; KH1's flinch lets it fall. `Strikes::hold`: a foe a
  holding blow hit is drawn, for 24 frames after each such hit, toward a point no more than 90
  from Sora at his height + 30, through KH1's per-frame push (`+0x120`, at most 30 a frame).

