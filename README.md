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
  map-collision/   engine-agnostic: moving a body against level polygons the way KH2 does
                   (sphere sweep, floor ray, step-up, slopes, ceilings)
  kh2-camera/      engine-agnostic: KH2's follow and lock-on camera
  combat-core/     engine-agnostic hit rules: damage, resistances, clamps, guard, shapes, the
                   per-target hit record (hit once, re-hit intervals, rows by hit count)
  character-core/  engine-agnostic 60 Hz controller: run/turn, jump, High Jump, Glide, Dodge Roll,
                   Quick Run, Aerial Dodge, Guard, landing recovery, lock-on, ground/air combos
  kh2-formats/     readers: Epic HED/PKG, BAR, 00battle.bin (incl. PTYA), 03system.bin pref,
                   MSET/ANB motions + root curves + triggers, model collision,
                   MDLX skeletons, full keyframed motions, objentry, sklt
  kh2-anim/        poses a skeleton from a motion like the game does: F-curves, constraints, IK
  kh2-bdx/         runs KH2's BDX scripts (the bytecode inside spell and object files)
  kh2-magic/       spells run from their own files' scripts: casting, attacks, effects
  kh2-data/        finds your install and builds the controller and attack data for Sora
  kh2-combat-cli/  `kh2-combat`: dump tables, simulate hits, run scripted movement checks
  kh2-bevy/        `kh2-playground`: Bevy app with an animated mannequin, training dummies, hitbox gizmos
docs/
  combat-model.md    damage rules: source and confidence for each
  movement-model.md  movement/moveset rules: source and confidence for each
  animation.md       how the skeleton is posed (from the executable's motion evaluator)
  magic-model.md     how spells run: casting, the script machine, every trap the spells use
  camera-model.md    the field and lock-on camera
  drive-model.md     Drive Forms: gauges, the change, Anti, levels, Wisdom's shots
  reaction-commands.md  how reaction commands are offered and run (the Soldier's Cyclone)
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
   - `--cam-distance N`: the camera's default distance (the game's is 420)
   - `--manual-camera`: no automatic swing-behind
   - `--enemies N`: how many Soldiers (default 1)
   - `--drive-max N`: Drive gauge bars (default 9, starts full)
   - `--form-level N`: every form's level (default 7)
   - `--hp N`: Sora's max HP (default 60; the real value depends on story bonuses)
   - `--magic-level 1|2|3`: Fire/Fira/Firaga and so on (default 3)

### Playground controls

| Action | Keyboard / mouse | Xbox pad | PlayStation pad |
|---|---|---|---|
| Move (Alt = walk) | WASD | left stick | left stick |
| Jump (hold for full height); in the air: Aerial Dodge | Space | B | Circle |
| Attack | J / left click | A | Cross |
| Guard (standing) / Dodge Roll or Quick Run (moving; hold to keep dashing); in the air: hold to Glide | K / Shift | X | Square |
| Lock-on / switch target | Tab / middle click; Z, C | RT; R2 + right stick | R1; R2 + right stick |
| Camera turn / zoom / reset | Q/E, right-drag / mouse up-down / F | right stick / R3 | right stick / R3 |
| Reaction command | L | Y | Triangle |
| Command menu: move / confirm / back out | Up, Down or mouse wheel / J / Space | D-pad / A / B | D-pad / Cross / Circle |
| Shortcut menu: hold, then press a face button to cast its spell (in the Magic list: assigns the highlighted spell) | hold Ctrl + L / K / Space / J | hold LB + Y / X / B / A | hold L1 + Triangle / Square / Circle / Cross |
| Drive: Valor, Wisdom, Limit, Master, Final / Revert | 7 8 9 0 - / Backspace | command menu | command menu |
| Cast directly (Fire, Blizzard, Thunder, Cure, Magnet, Reflect) | 1–6 | | |
| Refill the Drive gauge (testing; the form gauge while in a form) | F5 | Back | Select / Share |
| Send the enemies away / bring them back (out of combat: relaxed idle, walk and run, Keyblade put away) | F6 | | |
| Ability menu (equip with J/Enter, U = unlimited AP) | Esc / P | Start | Start / Options |
| Attack/guard shapes, hit shapes, skeleton, other collision / reset position | F1 F2 F3 F4 / R | | |

The shortcuts start as Fire, Blizzard, Thunder and Cure.

Jump at a wall that is slightly too high, holding toward it, to grab the ledge (push toward it to climb, pull away to drop).

The arena is made of polygons and runs the game's own map collision. It has:
- stairs with 20-unit steps
- a 20° ramp, and a 45° ramp that's too steep to climb
- a floating slab to bonk your head on
- a normal-jump ledge, a High Jump ledge, and a tall tower to glide from

The enemies are Soldiers (M_EX010) built from your install: their own skeleton and motions (drawn as a
dark stick figure), hurt shapes, attacks, HP and stats.
- **When they're hit** they flinch, slide or get launched by the game's reaction rules.
- **They come at you** with a simple timer of ours (KH2's enemy scripts aren't run). Their
  claws can hit Sora, and his Guard bounces them off.

**Magic** runs from the spell files' own scripts (`docs/magic-model.md`). Each effect shows
as a glowing orb at its real position, and each spell attack as its real hit volume.

**Drive Forms** use each form's own motions, weapons, movement and combo table. Wisdom fires
its shots from its model's script (`docs/drive-model.md`).

**Reaction commands:** while a Soldier does its spinning attack, Cyclone appears above the
command menu. Press L / Y (`docs/reaction-commands.md`).

Without the Soldier's files the playground falls back to plain column dummies.

## Using it in another game

Depend on `character-core` and `combat-core`. Describe moves as `Clip`s (length, root motion,
windows) and `ComboRow`s, then call `Character::step` at 60 Hz. You handle collision; the
controller returns a displacement, facing, active hitboxes and invulnerable/guard flags. To use
KH2's numbers, add `kh2-data` and call `Install::locate(path)?.load_sora(Source::Live)` on an
install the player owns.

## KH2 combat in Kingdom Hearts Final Mix (in progress)

`crates/kh1-hook` puts KH2's movement, combos and hit reactions on Sora in KH1 Final Mix (PC,
via OpenKH's Panacea), with KH2's animations retargeted onto KH1's Sora, while magic, summons,
specials and KH1's own moves stay KH1's. Install, the `kh1-kh2.ini` switches, the ability
mapping and the roadmap: `docs/kh1-port.md`.

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
  the exact difficulty table (see `docs/combat-model.md`). Which ATKP row applies depends on
  how many hits in a row the target has taken from that attack group, as in the game.
- **Menus:** the command menu, ability names, AP costs and the level's AP total, spell names,
  MP costs and Sora's MP come from 03system, 00battle and the message files.
- **Spells:** every spell's behaviour comes from its own script, with its sizes and timings
  from 03system `magi`.

How the executable was studied: Ghidra headless on your own copy, kept outside the repo. Only
plain-language notes are in `docs/`.

## Still approximated or open (see `docs/movement-model.md`)

- **Stick tilt → walk/run, landing recovery, and roll / Quick Run cancel points.**
- **Air-control target speed and invulnerability timer units.**
- **Animation:** feet don't adapt to slopes and stairs, and there's no head look-at.
- **Enemy AI:** the Soldiers use our own timer, not their scripts.
- **MP Charge** refills over a fixed 10 seconds (the game's rate isn't read yet).
- **Not done yet:** items, limits, summons, other enemies' reaction commands, form keyblade
  stats.

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
