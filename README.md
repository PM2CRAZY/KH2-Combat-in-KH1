# kh2-combat

A Rust re-implementation of **Kingdom Hearts II Final Mix's movement and combat**, built as
libraries you can drop into other games, plus a Bevy playground with a plain capsule rig.

Every number comes from **your own install** at runtime: speeds, jump heights, growth
abilities, each move's length and root motion, hitbox and combo windows, the combo table and
attack power. This repository contains no game files, no extracted data and no decompiled code.

```
crates/
  combat-core/     engine-agnostic hit rules: damage, resistances, clamps, guard, multi-hit gating
  character-core/  engine-agnostic 60 Hz controller: run/turn, jump, High Jump, Glide, Dodge Roll,
                   Quick Run, Aerial Dodge, Guard, landing recovery, lock-on, ground/air combos
  kh2-formats/     readers: Epic HED/PKG, BAR, 00battle.bin (incl. PTYA), 03system.bin pref,
                   MSET/ANB motions + root curves + triggers, model collision
  kh2-data/        finds your install and builds the controller and attack data for Sora
  kh2-combat-cli/  `kh2-combat`: dump tables, simulate hits, run scripted movement checks
  kh2-bevy/        `kh2-playground`: Bevy app with a capsule rig, training dummies, hitbox gizmos
docs/
  combat-model.md    damage rules: source and confidence for each
  movement-model.md  movement/moveset rules: source and confidence for each
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
   - `--gravity G`
   - `--backup`: read the untouched archives
   - `--synthetic`: made-up numbers, no install needed

### Playground controls

| Action | Keyboard / mouse | Xbox pad |
|---|---|---|
| Move (Alt = walk) | WASD | left stick |
| Jump (hold in the air to Glide) | Space | B |
| Attack | J / left click | A |
| Guard (standing) / Dodge Roll or Quick Run (moving) / Aerial Dodge (air) | K / Shift | X |
| Lock-on | Tab / middle click | RT |
| Camera | Q/E, arrows, right-drag | right stick |
| Growth levels: High Jump, Quick Run, Aerial Dodge, Glide, Dodge Roll | 1 2 3 4 5 | |
| Combo Plus / Air Combo Plus / Finishing Plus | 6 7 8 | |
| Hitbox gizmos / reset position | F1 / R | |

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
  - Dodge Roll: 10 / 16 / 24 / 32 invulnerable frames, plus its 438-unit root-motion roll
  - Glide: speeds, turn rate and descent limits
  - Quick Run and Aerial Dodge: speeds, times and heights
- **83 of Sora's motions:** their exact lengths, root motion (attack lunges, the roll), combo-input
  windows, finisher-chain windows, turn-to-target windows, and Keyblade hitbox windows with ATKP
  ids. Guard comes from the guard-type ATKP window.
- **Combo choice:** Sora's 37 PTYA rows, picked by distance and height to the target. For
  example, a 250-unit target gets the A302 dash attack.
- **Damage per hit:** ATKP power × (Strength − Defense) × resistance, clamped per enemy
  (see `combat-model.md`).

## Approximated or unconfirmed (see `docs/movement-model.md`)

- **Gravity.** No table holds it. It's estimated at 0.547 from the jump animation length;
  override it with `--gravity`.
- **How the combo table is read.** Row kinds, the finisher flag and tie-breaks are inferred
  from the data's patterns. Without a target the ground combo comes out A300 → A300 → A304;
  if the real game alternates swings, the `sub` field is probably the missing link.
- **Glide descent formula, roll cancel point, landing recovery length, and the approach
  assist before the first hitbox.**
- **No skeleton.** Hitboxes use the Keyblade's real sphere sizes, but they're swept in a
  fixed arc in front of Sora rather than following the animated blade.
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
