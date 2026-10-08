# kh2-combat

A Rust re-implementation of **Kingdom Hearts II Final Mix's combat rules**, packaged as a
library you can drop into other games. The library reads KH2's combat tables from **your own
install** at runtime. This repository contains no game files, no extracted data and no
decompiled code.

```
crates/
  combat-core/     engine-agnostic rules: damage, healing, resistances, clamps, reactions,
                   knockback, multi-hit gating, revenge, drive/HP drain. No deps, no KH2 data.
  kh2-formats/     readers for the Epic HED/PKG archives, BAR containers, 00battle.bin tables
  kh2-data/        locates your install, loads the tables, converts rows into combat-core types
  kh2-combat-cli/  `kh2-combat` debug tool: dump tables, simulate a hit step by step
docs/combat-model.md   every rule, where it came from, and how sure we are
```

## Using it in another game

Depend only on `combat-core` and fill `AttackDef` / `Combatant` from your own data:

```rust
use combat_core::*;
let slash = AttackDef { power: 1.0, hit_interval: 0, ..AttackDef::default() };
let player = Combatant::new(1, Team::Player, Stats { hp: 100, strength: 30, magic: 10, defense: 5 });
let mut foe = Combatant::new(2, Team::Enemy, Stats { hp: 80, strength: 12, magic: 4, defense: 6 });
let mut tracker = HitTracker::new();
// Call this every frame your hitbox overlaps `foe`; `swing` is a per-swing id.
let out = tracker.try_hit(frame, swing, &DamageRules::default(), &slash, &player, &mut foe);
// Then play out.reaction, apply out.knockback, add out.attacker_drive_gain, ...
```

To use KH2's real numbers, add `kh2-data` and point it at a KH2 install the player owns:

```rust
let data = kh2_data::Install::locate(r"D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe")?.load()?;
let thundaga = data.attack(249, 256).unwrap();
let data_saix = data.enemy(253, None).unwrap();
```

The crate is engine-agnostic: no Bevy, no physics library. You do collision; it decides what a hit does.

## CLI

```
cargo build --release
kh2-combat info --game "D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe"
kh2-combat enmp --game ... --id 253
kh2-combat hit  --game ... --attack 249:256 --enemy 253 --mag 80 --difficulty critical
```

`--backup` reads the untouched archives that the OpenKH Mods Manager keeps in `Image/en/backup/`.
On your install, 00battle.bin is identical in both copies.

## What works

- Reading `kh2_first.hed/.pkg` straight from the install: MD5 name lookup, decryption, zlib.
  The cipher's 256-byte table is found inside your executable at runtime and checked by
  SHA-256; it is never stored in the repo.
- Parsing 00battle.bin: ATKP (2713 attacks), ENMP (229 enemies), LVPM, PLRP, LVUP (13
  characters × 99 levels), FMLV, MAGC, BTLV. All checked against your install.
- `combat-core`:
  - the damage pipeline (stat − defense → power → element → boost → difficulty → clamp → minimum)
  - healing, guard and armor-pierce, target filtering
  - per-swing hit gating with ATKP's `interval`
  - revenge build-up, drive/HP drain outputs
- 18 tests (unit + doc), plus opt-in tests against a real install (`KH2_GAME=... cargo test`).

## What's missing

- **In-game confirmation of three hypotheses** (see `docs/combat-model.md`):
  1. ENMP min/max damage: fraction of max HP (default) or a raw clamp?
  2. Which side `critical_adjust` applies to.
  3. Whether enemy Strength/Defense come from LVPM.

  Suggested test: on Critical, hit Data Saïx with Thundaga at a known Magic and compare the
  number against `kh2-combat hit`.
- Hitbox timing and Sora's combo tree: needs an MSET/ANB motion-trigger reader (next milestone).
- PTYA (party combos), LIMT/SUMN logic, PRZT drops, PATN, STOP, VBRT: not parsed yet.
- Enemy AI, reactions as animations, knockback physics: left to the host engine.
- ATKP element ids 4–6 are mapped to Dark/Light/Neutral by position. Element 5 appears on
  392 rows, so its meaning needs checking.

## Rules this repo follows

- No game assets, dumps or decompiled code are committed. A whitelist `.gitignore` enforces
  it, and scratch output goes in `local/` (ignored).
- Single-player, offline, for a copy you own. No DRM or anti-cheat is touched.
- Credits and licences: `CREDITS.md`, `NOTICE`.

## Licence

`combat-core` and the CLI: MIT OR Apache-2.0. `kh2-formats` and `kh2-data` contain ports of
OpenKH code: Apache-2.0. See `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`.
