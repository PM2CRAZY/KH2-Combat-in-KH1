# KH2 combat in Kingdom Hearts Final Mix (PC)

`crates/kh1-hook` builds `dinput8.dll`, which OpenKH's Panacea loads into
`KINGDOM HEARTS FINAL MIX.exe` (KH 1.5+2.5 ReMIX, Epic). It hands Sora to the KH2 controller
in `character-core` (movement, combo, reactions) with KH2's own animations, retargeted onto
KH1's Sora. The reverse-engineering notes behind each stage are in `AGENTS.md` (KH1 stages).

## Install

1. Build: `cargo build --release -p kh1-hook --target x86_64-pc-windows-gnu` (see
   `AGENTS.md` for the cross-compile flags).
2. Copy the DLL to an OpenKH mod folder for KH1 as `kh1-kh2.dll` next to
   `crates/kh1-hook/openkh/mod.yml`, enable the mod, build and patch in the Mods Manager.
3. The game reads the KH2 data it needs from the KH2 install in the same folder. It writes
   `kh1-kh2.log` next to the game.

## Settings: `kh1-kh2.ini`

Next to the game; written with the defaults when missing, read when the game starts.

| Key | Values | What it does |
|---|---|---|
| `abilities` | `test` (default), `kh1`, `all` | `test`: every KH2 movement ability at MAX, Guard, Aerial Recovery, Combo Master, no combo modifiers. `kh1`: KH2's abilities from KH1's Ability menu and Sora's level (below). `all`: everything, every combo modifier. |
| `hurt` | `kh2` (default), `kh1` | How Sora takes hits while KH2 has him. |
| `carry` | `on` (default), `off` | Moving platforms carry him, KH1's pushes move him. |
| `ceilings` | `on` (default), `off` | Ceilings stop KH2's jumps. |
| `grabs` | `on` (default), `off` | KH1's ledge, vine, bar and swim moves take him from KH2's falls. |
| `commands` | `on` (default), `off` | KH1's own commands (magic, items, specials, summons) take him from KH2. |
| `swing_fx` | `on` (default), `off` | KH2's swings play KH1's Keyblade trail, swing effect and Sora's voice. |

## What is KH1's and what is KH2's

- KH2: walking, running, jumping, High Jump, Quick Run, Aerial Dodge, Glide, Dodge Roll,
  Guard, the ground and air combos (KH1's Attack command is KH2's attack button), lock-on
  (KH1's R1 target), hit reactions, repels off guarding enemies.
- KH1: damage numbers (KH1's attack data and stats), enemies, magic, summons, items,
  specials (Sonic Blade and the rest), ledge grabs, vines, swimming, doors and zone changes,
  events, death.

## Abilities from KH1's menu (`abilities = kh1`)

KH1's own abilities, while equipped:

| KH1 | KH2 |
|---|---|
| High Jump (levels 1 / 2 / 3) | High Jump LV1 / LV2 / MAX |
| Glide, each Superglide | Glide LV1, a level more each (to MAX) |
| Guard | Guard |
| Dodge Roll | Dodge Roll LV1 (higher levels with Sora's level, below) |
| Combo Plus, Air Combo Plus (each) | Combo Plus, Air Combo Plus (each) |
| Vortex | Flash Step |
| Aerial Sweep | Aerial Sweep |
| Counterattack | Counterguard |
| Blitz | Finishing Leap |
| Slapshot | Slapshot |
| Sliding Dash | Slide Dash |
| Hurricane Blast | Explosion |
| Ripple Drive | Vicinity Break |
| Stun Impact | Guard Break |
| Gravity Break | Aerial Finish |

Final Mix's own Combo Master is KH2's Combo Master while equipped.

What only KH2 has is added to KH1's Ability menu as new abilities, learned at Sora's level
(added when he reaches it, or when the game loads if he is already past it):

| Level | Ability | Menu | AP |
|---|---|---|---|
| 4 | Aerial Recovery | Sora's | 2 |
| 7 | Quick Run (on when learned) | shared | - |
| 10 | Dodge Slash | Sora's | 2 |
| 13 | Aerial Dodge (on when learned) | shared | - |
| 16 | Horizontal Slash | Sora's | 2 |
| 20 | Negative Combo | Sora's | 2 |
| 25 | Aerial Spiral | Sora's | 2 |
| 31 | Upper Slash | Sora's | 4 |
| 34 | Finishing Plus | Sora's | 5 |
| 43 | Retaliating Slash | Sora's | 3 |
| 48 | Aerial Dive | Sora's | 2 |
| 50 | Magnet Burst | Sora's | 3 |

Quick Run and Aerial Dodge reach LV2 / LV3 / MAX at levels 22 / 40 / 65 and 28 / 46 / 70;
the dodge roll (KH1's Dodge Roll equipped) reaches LV2 / LV3 / MAX at 19 / 37 / 60.

Sora's ability list holds 48 (KH1's most). When it is full, KH2's abilities come first: a
KH1 ability that is off gives up its place, a spare copy first (an ability he has more than
once), then the least missed (Encounter Plus, EXP Zero, Cheer, Scan, Evolution, Jackpot, Lucky
Strike, Treasure Magnet, ...). Equipped abilities, Guard, Dodge Roll, the specials and the
combo modifiers are never removed. Each removal is written to `kh1-kh2-removed.txt` next to the
game.

They live in an extended copy of KH1's ability table (ids 66 to 77), so a save holding them
needs this mod to show them. To go back to a plain KH1 save: set `abilities = test`, load the
save (they are taken out of Sora's lists), save, then remove the mod.

The log shows what KH1's menu gives every time it changes (`KH1's menu gives KH2: ...`) and
each ability learned (`Sora learns ...`).

## Roadmap

1. Loading. Done.
2. Animations. Done.
3. Movement. Done.
4. Combat parity. Done.
5. Taking hits: dodge-roll invulnerability, KH2's guard, KH2's reactions on KH1's damage.
   Built (stage 28, review fixes 32), to be tried in game.
6. Movement polish: platforms and pushes (29), ceilings (30), KH1's ledge / vine / bar /
   swim moves (31, 32). Built, to be tried in game.
7. Abilities from KH1's menu (29). Built (`abilities = kh1`); KH2-only abilities come with
   Sora's level until they are added to KH1's menu.
8. What stays KH1's: KH1's commands hand Sora to KH1, in the air through KH1's own fall (33).
   Built, to be tried in game.
9. Release polish: KH1's Keyblade trail, swing effects and voice on KH2's swings (34).

## Trying the new stages

- Taking hits: let an enemy hit Sora standing, while attacking, in the air; dodge-roll
  through an attack (no damage); guard facing an attack (no damage, KH2's guard repel), then
  with the attack from behind (damage). The log has a line per hit (`Sora hit by ...`).
- Platforms: ride a moving platform (`carried ...` lines in the log).
- Ceilings: jump under a low ceiling (`ceiling: rose ...`).
- Grabs: fall past a ledge holding toward it; jump at a vine (`KH1 takes him: ...`).
- Commands: cast a spell standing and in mid-air, use an item, a special
  (`KH1 takes him: command ...`, then `KH1's move ended ...: KH2 again`).
- Swings: the Keyblade trail and Sora's voice on KH2's combo (`Keyblade trail drawn ...`).
- Abilities: set `abilities = kh1`, start the game, open and change the Ability menu.

Any of these misbehaving can be switched off in the ini without a new build.
