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
| `kh2_sounds` | `magnet_explosion` (default), `all`, `off` | Which moves play KH2's own sounds (its effect and Keyblade sounds, from the KH2 install). |
| `kh2_volume` | `1.0` (default) | Their volume. |
| `kh2_voice` | `off` (default), `on` | `on`: Sora's KH2 battle voice on those moves; `off`: KH1's voice. |
| `kh2_effects` | `magnet_explosion` (default), `all`, `off` | Which moves show KH2's own visual effects (its particle effects, from the KH2 install, drawn in KH1's frame). |
| `flip_weapons` | KH1 item ids, `0x53` (default), `none` | Weapons KH2's grip holds the wrong way round, turned half round in his hand (the Dream Shield). |
| `fx_test` | `off` (default), `on` | Starts Magnet Burst's effect at Sora every 4 seconds, to check the effect renderer. |
| `bonus_ap` | `on` (default), `off` | With `abilities = kh1`: Sora's max AP grows by the AP of each KH2 ability he learns (below). |
| `auto_aim` | `off` (default), `on` | Off: KH2's swings turn toward and home in on only KH1's own target, the R1 lock or the small cursor's soft target. On: KH2's own pick, the nearest enemy in reach. |
| `wall_clink` | `off` (default), `on` | On: a swing that meets a wall clinks off it (KH1's background ray) and bounces Sora back. Off: swings pass through walls. |
| `camera` | `kh2` (default), `kh1` | KH2's field camera (follows behind, swings behind while running with Auto, right stick turns and zooms, R3 resets, KH2's lock-on framing) or KH1's own. The Config menu's new last row, Camera Style, switches it in game and writes it here. |
| `camera_scale` | `0.89` (default) | KH2's camera distances and heights are multiplied by this. 0.89 is KH1 Sora's size against KH2 Sora's (hips 78.9 / 88.5), so he is framed as in KH2. |
| `camera_lower` | `0` (default) | KH2's camera, look point and eye together, this much lower. 0 = as KH2. |
| `config_row` | `on` (default), `off` | The Config menu's Camera Style row (off: none; `camera` alone decides). |
| `shortcuts` | `4` (default), `3` | KH2's four shortcuts: L1 + the cancel button (Circle) is the fourth, set in Customize. `3`: KH1's three. |

## What is KH1's and what is KH2's

- KH2: walking, running, jumping, High Jump, Quick Run, Aerial Dodge, Glide, Dodge Roll,
  Guard, the ground and air combos (KH1's Attack command is KH2's attack button), lock-on
  (KH1's R1 target), hit reactions, repels off guarding enemies.
- KH1: damage numbers (KH1's attack data and stats), enemies, magic, summons, items,
  specials (Sonic Blade and the rest), ledge grabs, vines, swimming, doors and zone changes,
  events, death.

## Abilities from KH1's menu (`abilities = kh1`)

KH1's own abilities, while equipped (those that are KH2 moves show their KH2 name and help in
the menu, e.g. Hurricane Blast is listed as Explosion):

| KH1 | KH2 |
|---|---|
| High Jump | High Jump LV1 (higher levels with Sora's level, below) |
| Glide | Glide LV1, LV2 at level 40, LV3 at 52; MAX once Chernabog is beaten |
| Superglide | none: turned into Glide (its Chernabog reward reads "Glide MAX") |
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
(added when he reaches it, or when the game loads if he is already past it, off until equipped;
a "New Ability" box in KH1's level-up prompt style names it):

| Level | Ability | Menu | AP |
|---|---|---|---|
| 4 | Aerial Recovery | Sora's | 2 |
| 10 | Dodge Slash | Sora's | 2 |
| 12 | Quick Run LV1 (LV2 at 20, LV3 at 32, MAX at 45; its menu name shows the level, an "Ability Up" box each time) | Sora's | 2 |
| 16 | Horizontal Slash | Sora's | 2 |
| 20 | Negative Combo | Sora's | 2 |
| 25 | Aerial Spiral | Sora's | 2 |
| 34 | Finishing Plus | Sora's | 5 |
| 43 | Retaliating Slash | Sora's | 3 |

Bosses give KH2 moves too, as KH2 does (the user's list), on top of what they already give;
learned once the world's story progress passes the boss (off until equipped, with a "New
Ability" box):

| Boss | Ability | AP |
|---|---|---|
| Sabor | Upper Slash | 4 |
| Clayton | Aerial Sweep (KH1's) | KH1's |
| Jafar | Aerial Finish (KH1's Gravity Break) | KH1's |
| Pot Centipede | Guard Break (KH1's Stun Impact) | KH1's |
| Dragon Maleficent | Explosion (KH1's Hurricane Blast) | KH1's |
| Captain Hook | Magnet Burst | 3 |
| Cerberus | Aerial Dive | 2 |
| Genie Jafar (Agrabah) | Aerial Dodge LV1 (higher levels below) | 3 |

Earlier than KH1's level-ups (Dream Shield: 45 / 39 / 69 / 78 / 60), if Sora doesn't have them yet: Counterguard at 22, Slide Dash at 30, Vicinity Break at 36, Finishing Leap at 48; Flash Step comes for beating Sabor. Aerial Sweep also comes at level 28 if Clayton hasn't given it yet (KH1's Dream Shield gives it
only at 84). Guard Break stays at KH1's own level 18 (Dream Shield).

Each of these Sora learns raises his max AP by its own AP (`bonus_ap`), so the KH2
abilities take none of the AP KH1 gives him: +2 at level 4, +32 in all by level 50 (KH1's
cap of 100 still holds). The save keeps KH1's own AP; the bonus is added while the game runs.

The dodge roll (KH1's Dodge Roll equipped) reaches LV2 / LV3 / MAX at levels 19 / 37 / 60; its
menu name shows the level ("Dodge Roll LV2"), with an "Ability Up" box each time.
High Jump reaches LV2 / LV3 / MAX at levels 30 / 44 / 58, and Aerial Dodge at 30 / 42 / 54,
the same way (name with its level, "Ability Up" box), and Glide at 40 / 52 to LV3, MAX for
beating Chernabog (End of the World progress 0x33, KH1's Superglide reward, renamed "Glide
MAX"; Superglide itself is turned into Glide or taken out).
Sora keeps one High Jump and one Glide: spare copies (a randomizer can give
more) are taken out of the shared list when the game loads or he gets one, and noted in
`kh1-kh2-removed.txt`.

Sora's ability list holds 48 (KH1's most). When it is full, KH2's abilities come first: a
KH1 ability that is off gives up its place, a spare copy first (an ability he has more than
once), then the least missed (EXP Zero, Cheer, Scan, Evolution, Jackpot, Lucky
Strike, Treasure Magnet, ...). Equipped abilities, Guard, Dodge Roll, the specials and the
combo modifiers are never removed. Each removal is written to `kh1-kh2-removed.txt` next to the
game.

They live in an extended copy of KH1's ability table (ids 66 to 77), so a save holding them
needs this mod to show them. To go back to a plain KH1 save: set `abilities = test`, load the
save (they are taken out of Sora's lists), save, then remove the mod.

The log shows what KH1's menu gives every time it changes (`KH1's menu gives KH2: ...`) and
each ability learned (`Sora learns ...`).

## Roadmap

Done (played by the user):

1. Loading, animations, movement, combat parity.
2. Taking hits the KH2 way (28, 32; the hit pass's second argument fixed in 46).
3. Movement polish: platforms, pushes, ceilings, KH1's ledge / vine / bar / swim moves.
4. KH1's commands (magic, items, specials, summons) take Sora from KH2 and give him back,
   jump casts and the L1 shortcut included (33, 37).
5. Abilities from KH1's menu with KH2's first (29, 36, 38), Encounter Plus kept, extra AP
   for the KH2 abilities (43), one High Jump (LV3), Glide LV3 / Superglide MAX, Quick Run LV2 and
   Aerial Dodge LV3 (47, 48), both later with AP in Sora's list.
6. Swing feel: KH1's Keyblade trail, swing effects and voice (34); KH2's own sounds for
   Magnet Burst and Explosion, with KH1's voice (40-42).

Built, being checked in game:

7. KH2's own visual effects for Magnet Burst and Explosion, drawn in KH1's Direct3D 12
   frame (44-46): placement on Sora and the Keyblade, depth against walls and enemies,
   brightness of the blend modes, high frame-rate (interpolated) mode.
8. Magnet Burst's pull on enemies (39).

Next:

9. A hook audit: every hooked or called KH1 function's arguments and return checked against
   the executable. Done in 49: the set-motion hooks (`0x14029D7F0`, `0x14029D8A0`) and the
   ground-IK skeleton pass (`0x140392710`) take a float third argument (XMM2) that the hooks
   passed as an integer; fixed.
10. Release: the OpenKH mod package with a default ini (`abilities = kh1`), install notes,
    and the KH2 install found next to KH1 or set in the ini.

Skipped (the user): KH2's effects for the rest of Sora's moves, KH2's hit effects and sounds
on enemies.

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
