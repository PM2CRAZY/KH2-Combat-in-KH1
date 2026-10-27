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
| `kh2_volume` | `1.0` (default) | Their volume (written into the cut banks' records; KH1's sliders do the rest). |
| `kh2_voice` | `off` (default), `on` | `on`: Sora's KH2 battle voice on those moves; `off`: KH1's voice. |
| `kh2_effects` | `magnet_explosion` (default), `all`, `off` | Which moves show KH2's own visual effects (its particle effects, from the KH2 install, played by KH1's own effect system). |
| `flip_weapons` | KH1 item ids, `0x53` (default), `none` | Weapons KH2's grip holds the wrong way round, turned half round in his hand (the Dream Shield). |
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

Since the split (testing, `docs/abilities.md`), each ability id has one owner and one
meaning. KH1's abilities are KH1's own again, with KH1's names, help and effect: Hurricane
Blast is KH1's Hurricane Blast. KH2's abilities have their own ids, 66 to 87, and come with
KH2's names and AP. KH1's abilities that both games have with the same job also work for KH2
while equipped:

| KH1 | KH2, while KH2's controller has Sora |
|---|---|
| High Jump | High Jump: LV2 / LV3 / MAX at levels 30 / 44 / 58 |
| Glide | Glide: LV2 at level 40, LV3 at 52 |
| Superglide | Glide MAX |
| Guard | Guard |
| Dodge Roll | Dodge Roll: LV2 / LV3 / MAX at levels 19 / 37 / 60 |
| Combo Plus, Air Combo Plus (each) | Combo Plus, Air Combo Plus (each) |
| Combo Master | Combo Master |

KH1's combo modifiers (Vortex, Aerial Sweep, Counterattack, Blitz, Slapshot, Sliding Dash,
Hurricane Blast, Ripple Drive, Stun Impact, Gravity Break) are KH1's: KH1's own combo runs
them, and they do nothing for KH2's combo yet.

KH2's abilities are in KH1's Ability menu as their own, off until equipped. Since the testing
branch's vanilla progression (the tester, what-is-still-missing after m17: "make it all vanilla,
just give the player all the custom abilities we added, existing saves get updated, remove extra
ap for now") all of them are Sora's from the start, KH1 gives its own abilities on its own
schedule, and no KH2 ability ever takes a KH1 one's place (2 places are always left for KH1's own
grants; ours give theirs back when KH1's fill the list). There is no extra AP. KH2's growth
levels come only from the Drive Forms' levels. The details, and how older saves are put back,
are in `docs/abilities.md`.

They live in an extended copy of KH1's ability table (ids 66 to 87), so a save holding them
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
