# kh2-combat

A Rust re-implementation of **Kingdom Hearts II Final Mix's movement and combat**, built as
libraries you can drop into other games, plus a Bevy playground where Sora is a plain
mannequin that plays his real animations.

Every number comes from **your own install** at runtime: speeds, jump heights, growth
abilities, each move's length and root motion, hitbox and combo windows, the combo table and
attack power, and Sora's skeleton and animations. This repository contains no game files, no
extracted data and no decompiled code, and the playground loads no meshes or textures.

```
crates/
  combat-core/     engine-agnostic hit rules: damage, resistances, clamps, guard, multi-hit gating
  character-core/  engine-agnostic 60 Hz controller: run/turn, jump, High Jump, Glide, Dodge Roll,
                   Quick Run, Aerial Dodge, Guard, landing recovery, lock-on, ground/air combos
  kh2-formats/     readers: Epic HED/PKG, BAR, 00battle.bin (incl. PTYA), 03system.bin pref,
                   MSET/ANB motions + root curves + triggers, model collision,
                   MDLX skeletons, full keyframed motions, objentry, sklt
  kh2-anim/        poses a skeleton from a motion like the game does: F-curves, constraints, IK
  kh2-data/        finds your install and builds the controller and attack data for Sora
  kh2-combat-cli/  `kh2-combat`: dump tables, simulate hits, run scripted movement checks
  kh2-bevy/        `kh2-playground`: Bevy app with an animated mannequin, training dummies, hitbox gizmos
docs/
  combat-model.md    damage rules: source and confidence for each
  movement-model.md  movement/moveset rules: source and confidence for each
  animation.md       how the skeleton is posed (from the executable's motion evaluator)
```

## Testing it on Windows

1. Install Rust from <https://rustup.rs> and accept the Visual Studio C++ Build Tools prompt.
2. In this folder, run the tests. They need no game data.
   ```
   cargo test
   ```
3. Also check against your install (PowerShell):
   ```
   $env:KH2_GAME="D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe"; cargo test
   ```
4. Scripted movement and combo checks with your real data:
   ```
   cargo run --release -p kh2-combat-cli -- sora --game "D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe"
   ```
5. The playground (the first build takes several minutes):
   ```
   cargo run --release -p kh2-bevy -- --game "D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe"
   ```
   Options:
   - `--level N`: Sora's level, for stats from LVUP (default 30)
   - `--dummy-level N`
   - `--gravity G` (override the game's 20/49 for experiments)
   - `--backup`: read the untouched archives
   - `--synthetic`: made-up numbers, no install needed (capsule rig)
   - `--capsule`: draw the old capsule instead of the mannequin
   - `--cam-distance N`: camera distance (default 650)

### Playground controls

| Action | Keyboard / mouse | Xbox pad |
|---|---|---|
| Move (Alt = walk) | WASD | left stick |
| Jump; in the air: Aerial Dodge, then hold to Glide | Space | B |
| Attack | J / left click | A |
| Guard (standing) / Dodge Roll or Quick Run (moving) | K / Shift | X |
| Lock-on | Tab / middle click | RT |
| Camera | Q/E, arrows, right-drag | right stick |
| Growth levels: High Jump, Quick Run, Aerial Dodge, Glide, Dodge Roll | 1 2 3 4 5 | |
| Combo Plus / Air Combo Plus / Finishing Plus | 6 7 8 | |
| Attack spheres / hurt spheres / reset position | F1 / F2 / R | |

The arena has steps, a normal-jump ledge, a High Jump ledge and a tall tower to glide from.
The dummies are Shadows: their HP, resistances and damage limits come from ENMP/LVPM, and
hits go through `combat-core`.

## Using it in another game

Depend on `character-core` and `combat-core`. Describe moves as `Clip`s (length, root motion,
windows) and `ComboRow`s, then call `Character::step` at 60 Hz. You handle collision; the
controller returns a displacement, facing, active hitboxes and invulnerable/guard flags. To use
KH2's numbers, add `kh2-data` and call `Install::locate(path)?.load_sora(Source::Live)` on an
install the player owns.

## What's driven by data (your install)

- **Movement:** walk 2, run 8 units per frame. Jump 185 with 4 takeoff frames. Turning is
  capped at 15° per frame. Fall speed tops out at 16.
- **Growth abilities, LV1–MAX:**
  - High Jump: 235 / 310 / 385 / 585
  - Dodge Roll: 10 / 16 / 24 / 32 invulnerable frames, plus its 435-unit root-motion roll
  - Glide: speeds, turn rate and descent limits
  - Quick Run and Aerial Dodge: speeds, times and heights
- **83 of Sora's motions:** their exact lengths, root motion (attack lunges, the roll), combo-input
  windows, finisher-chain windows, turn-to-target windows, and Keyblade hitbox windows with ATKP
  ids. Guard comes from the guard-type ATKP window.
- **Combo choice:** Sora's 37 PTYA rows, read the way the game reads them:
  - first valid row in table order, with the anti-repeat `sub` swap
  - the combo count only advances on swings that connect
  - finishers when `count + offset ≥ max`
- **Physics from the executable:**
  - a 30 Hz velocity tick
  - double-sine jump arc with gravity 20/49
  - 0.9/0.95 air control
  - glide and Quick Run formulas
  - Aerial Dodge only after a real jump
- **Sora's skeleton and animations** (`docs/animation.md`): every motion posed the way the
  game's motion evaluator does it, including constraints and leg IK. The Keyblade's own
  animation is attached to his right hand (bone 178, from the `sklt` table). Attack and hurt
  spheres sit on the real bones, so hits follow the actual swing.
- **Damage per hit:** the game's own order and rounding, base-term clamps from ENMP/LVPM, and
  the exact difficulty table (see `docs/combat-model.md`).

How the executable was studied: Ghidra headless on your own copy, kept outside the repo. Only
plain-language notes are in `docs/`.

## Still approximated or open (see `docs/movement-model.md`)

- **Variable jump height.** Whether holding jump changes the height hasn't been traced yet.
- **Stick tilt → walk/run, landing recovery, and roll / Quick Run cancel points.**
- **Air-control target speed and invulnerability timer units.**
- **Animation blending** uses a fixed 8-frame cross-fade, not the game's per-move blend times.
  Feet don't adapt to slopes and stairs, and there's no head look-at.
- **Dummies don't attack yet.** Guard and invulnerability are shown (cyan / white) but never
  tested against incoming hits.
- **Not done yet:** magic, drive forms, limits, reaction commands, ledge hang, enemy AI.

## Rules this repo follows

- No game assets, dumps or decompiled code are committed. A whitelist `.gitignore` enforces
  it, and scratch output goes in `local/` (ignored).
- Single-player, offline, for a copy you own. No DRM or anti-cheat is touched. The archive
  cipher table is read from your exe and checked by hash, never stored.
- Credits and licences: `CREDITS.md`, `NOTICE`.

## Licence

`combat-core`, `character-core`, the CLI and the playground: MIT OR Apache-2.0.
`kh2-formats` and `kh2-data` contain ports of OpenKH code: Apache-2.0. See `LICENSE-MIT`,
`LICENSE-APACHE`, `NOTICE`.
