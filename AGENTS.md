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
