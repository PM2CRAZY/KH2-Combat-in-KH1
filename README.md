# KH2 Combat for KH1 (testing-merge branch)

A mod for **Kingdom Hearts Final Mix** on PC that gives Sora **Kingdom Hearts II's combat and
movement**: KH2's combos, finishers and combo abilities, Quick Run, Aerial Dodge, KH2's Glide and
Dodge Roll, Guard and Counterguard, KH2's hit reactions, KH2's field camera, a fourth shortcut,
KH2's Reflect, and the Drive Forms (Valor, Wisdom, Limit, Master, Final and Anti).

These are added to KH1's own engine. KH1's game does the work: its states, motion tables, effect
system, sound banks, menus and camera run the KH2 moves. Almost everything is a choice in KH1's
Config menu. On KH2 a setting aims to play exactly like KH2; on KH1 it plays as KH1 does.

This README is for the **testing-merge** branch: the testing branch, plus what the main branch
had (bonus AP, the main branch's growth abilities, Anti Form and more), rebuilt inside testing's
systems (`docs/testing-merge.md`). **Nothing on this branch has been checked in game yet.**

Sora's KH2 animations, sounds and effects are read from **your own** KH2 Final Mix files when the
game starts. This repository has no game files, no extracted data and no decompiled code.

## What you need

1. **KINGDOM HEARTS HD 1.5+2.5 ReMIX for PC, the Steam release**, with Kingdom Hearts Final Mix
   and Kingdom Hearts II Final Mix in the same install, as Steam installs them. The mod reads KH2
   straight from that install (`KINGDOM HEARTS II FINAL MIX.exe` next to `KINGDOM HEARTS FINAL
   MIX.exe`). You don't extract anything. If OpenKH's Mods Manager has backed up the original
   files, the mod reads the backups, so other KH2 mods don't leak in.
   - **Steam only.** The Epic Games Store release isn't supported on this branch. This is the
     owner's decision for now. The mod may load on Epic, but most of its features are written for
     the Steam build alone and are switched off there.
   - The mod checks which game build it's in. On a build it doesn't know, it changes nothing.
2. **OpenKH Mods Manager**, set up for KH 1.5+2.5 with **Panacea** installed:
   <https://github.com/OpenKH/OpenKh/releases>. On first run, its setup wizard asks which store your
   game is from and where your game folder is, and offers to install Panacea: say yes. Panacea is
   what loads this mod into the game. Steam's folder is usually
   `...\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5 ReMIX-`.

## Install

The mod is two files at the top of this branch:
- `kh1-kh2.dll`, the built mod. The branch's build bot updates it after every code change.
- `mod.yml`, which tells the Mods Manager to copy the DLL into the mod's `dll` folder for Panacea.

Steps:
1. **Back up your KH1 save** (recommended). Steam saves are in
   `Documents\KINGDOM HEARTS HD 1.5+2.5 ReMIX\Steam\<number>\`.
2. Download `mod.yml` and `kh1-kh2.dll` from the `testing-merge` branch. Put both in one zip, at the
   zip's top level (not inside a folder).
3. Open the OpenKH Mods Manager and pick **Kingdom Hearts Final Mix** (KH1) as the game.
4. Mods > Install a New Mod > **Select and install Mod Archive**, and choose the zip. Don't unzip it.
   - Installing by repository name (`PM2CRAZY/KH2-Combat-in-KH1`) may give you the repository's
     default branch instead of this one. For this branch, use the zip.
5. Tick **"KH2 Combat for KH1 (testing-merge branch)"** in the mod list.
6. Build > **Build and Run** (or Build Only, then start the game normally).
7. Load a save.

**The first start takes longer.** The mod builds its KH2 data (motions, sounds, effects) from your
files, and KH1's title waits on a loading screen while it does. The result is kept in a
`kh1-kh2-cache` folder next to the DLL. The first start after each new build makes it again.

The first start also writes the settings file, **`kh1-kh2.ini`**, in the game folder (the one with
`KINGDOM HEARTS FINAL MIX.exe`).

**Updating:** install the new zip the same way, then Build again. An old build left in the Mods
Manager's output folder keeps loading otherwise. To see which version is running, open Config in
the game and move to the mod's button (see "The Config menu"). Its help box ends with
"KH2 Combat version tm…".

## Controls

KH2's controls on KH1's pad. Only the changes are listed here. Magic, items, summons, KH1's
limits and its reaction commands (Triangle) stay KH1's own.

| Button | What it does |
|---|---|
| Attack | KH2's ground and air combos, finishers and combo abilities |
| Square, standing | Guard (KH1's or KH2's guard animation: Combat > Tweaks > Guard Animation). KH2's guard has its repel and Counterguard |
| Square, moving | Dodge Roll; held, Quick Run once Sora has it |
| Jump in the air | Aerial Dodge (once Sora has it, equipped) |
| Jump held in the air | Glide (KH2's with Movement > Glide at KH2), after the Aerial Dodge if he has it |
| L1 + face buttons | The shortcuts. **L1 + the cancel button** is KH2's fourth shortcut (set it in Customize) |
| Right stick | KH2's camera: turn and zoom. R3 resets it. It follows KH1's own camera up/down setting |
| D-pad left at the battle menu's top | Flips to the second page, as in KH2. Page 1: Attack, Magic, Items, Drive. Page 2: Attack, Summon, Party, Limit |
| Drive (battle menu) | The Drive Forms you own. In a form the same row is Revert |
| A secret code on the pad | Anti Form. The code stays a secret |

Optional, in the Config menu (all off by default):
- **R2 Camera Hold:** while R2 is held, the right stick doesn't move KH2's camera and R3 doesn't
  reset it.
- **L2 Stick Menu:** while L2 is held, the right stick moves the command menu like the d-pad and R3
  confirms, as in KH2.
- **Soft Reset:** hold L1 + L2 + R1 + R2 + Start + Select for a second to return to the title
  screen.

## The Config menu

KH1's Config menu gets one more button on its first page, above "Default settings". It's labelled
"KH1 in KH2" and opens the mod's lists. "Default settings" leaves the mod's settings alone. Each
row shows a help line. Your choices are kept in `kh1-kh2.ini`, not in the save. The defaults below
are a fresh ini's.

**Camera**
- Camera Style: KH1's camera or KH2's field camera (default KH2).
- R2 Camera Hold: see Controls (default Off).

**Combat**
- Physics: whose physics move Sora (run speeds, jump arc, gravity, air drift, attack travel)
  (default KH2). Drive Forms always use KH2's.
- Base Combo / Air Base Combo: KH1's or KH2's ground and air combo (default KH2).
- **Magic:** Fire, Blizzard, Thunder and Cure: cast KH1's or KH2's version of each spell (default
  KH1). Air Physics: how Sora moves while casting in the air (default KH2). Magic Combo: KH2 makes
  KH1's grounded casts a step of the combo, with Gravity and Stop as its finisher (default KH2).
  - Animations: KH1's or KH2's cast animation for Blizzard, Thunder, Cure, Gravity, Stop and Aero
    (default KH1).
- **Tweaks:**
  - KH1 Ability Chance (default Off): On keeps KH1's 30% chance on Stun Impact, Gravity Break and
    Zantetsuken.
  - Cast from Glide (default On): a KH2 spell ends the glide and is cast in the air.
  - Ledge Hop (default Off): an attack with the floor giving out ahead hops Sora off the ledge.
  - Guard Animation (default KH1).
  - Hit Reactions (default KH1): KH1's or KH2's animations when Sora is hit.
  - Criticals (default On): KH1's critical hits on every attack. Critical Plus needs it on.
  - Party Criticals (default Off): the same for Donald, Goofy and the guests.
  - Walls (KH1, Clink or Pass): what a KH2 swing does at a wall. KH1 sparks and goes on, Clink
    stops and bounces Sora back (KH2), Pass goes through. Until you pick one, it's Clink with
    Recoil at KH2 and KH1 otherwise.

**Movement**
- Jump, Glide, Dodge Roll: KH1's or KH2's for base Sora (default KH2).
- Misc Animations: standing, walking, running and items (default KH2).

**Command Menu**
- Drive Page: Drive on page 1 and Summon on page 2, as in KH2 (default), or the other way round.
- 2nd Page: On (KH2, default) means d-pad left flips the pages. Off (KH1) means there is no page 2.
- Cursor Memory and Command Lock: KH2's rules (default) or KH1's.
- L2 Stick Menu: see Controls (default Off).

**Drive**
- Form Cost: the Drive gauge, as in KH2 (default), or MP (3 MP, then the form drains MP).
- Gauge Refill (default KH2): leaving a world in a Drive Form refills the Drive gauge, HP and MP,
  and reverts.
- MP Refill (default Off): entering a form refills MP as well as HP.
- Recovery in Battle (default Off): Drive Recoveries can be used from the battle item slots.
  Off means the menu only, as in KH2.
- Party Required (default KH2): Valor needs Goofy, Wisdom needs Donald, Master, Final and Anti need
  both party slots filled, and Limit needs no one.
- **Limit:** Cost (KH2's 4 bars by default, or 3 bars). Trinity Limit (default Off): KH1's Trinity
  Limit in Limit Form. Ledges (default KH2): KH2's edge stop keeps a limit going on a ledge.
  Limits in Forms (default KH2): no KH1 limit in any Drive Form.
- **Anti:** Rules. KH2 (default): needs 3 bars and takes all 9, and Revert is greyed in battle.
  Balance (the main branch's rule): free, and no Revert.

**Progression**
- Story Unlocks (default Off): the Drive Forms open by the story (see Drive Forms). Off means all
  five from the start.
- Drive Gauge Growth (default Off): the gauge starts at 3 bars and grows by one for each of Sabor,
  Riku, Chernabog, Sephiroth, Kurt Zisa and the Unknown. Off means 9 bars.
- Drive Recovery (default On): Drive Recovery and High Drive Recovery are sold in item shops and
  dropped once a Drive Form is open, and found in chests.
- Reflect by Story (default On): Reflect, Reflera and Reflega come one per fight won: Pot
  Centipede, the Hercules Cup and Ice Titan. Off means they come with KH1's Aero levels.

**QoL**
- Soft Reset (default Off).
- Cutscene Skip (Off, Seen or All; default Off): skips the story cutscenes the pause menu can skip.
  Seen skips only the ones you've watched. Talks, shops and dialog you confirm line by line are
  never skipped.
- Item Pickup (default Vanilla): Skip means picking something up doesn't stop Sora.
- Chest Opening (Vanilla, Quick or Skip; default Skip, as KH2). Chest Animation (KH1 or KH2) only
  shows with Quick.

**Cosmetics**
- Keyblade Trail: Normal (on swings, as KH1) or Always.

**Misc**
- Battle Quotes: KH1's or KH2's (default KH1).
- Debug Menu: only there with `debug_menu = on` in the ini.

## The ini (`kh1-kh2.ini`)

The game folder's `kh1-kh2.ini` holds every setting, each with a comment that explains it. The
default file is in `crates/kh1-hook/src/win/settings.rs` (`DEFAULT_INI`). The mod reads it at start
and again whenever you save it, so most changes take effect in game. A few are only read at start.
Keys it lacks keep their defaults.

Some settings worth knowing:
- `abilities = kh1`: normal play. `remove` takes the KH2 abilities back out of your save (see
  Uninstall). `test` and `all` are for testing only.
- `kh2_effects`, `kh2_sounds`: which moves show KH2's own effects and play KH2's own sounds, or
  `off`.
- `damage_scale = 1.0`: multiplies the damage of Sora's KH2 attacks.
- `shortcuts = 3`: KH1's three shortcuts instead of four.
- `drive = off`: no second menu page and no Drive Forms.
- `reflect = off`: no KH2 Reflect.
- `experimental = on`: shows the **experimental settings** in the Config menu, and puts their lines
  at the bottom of the ini: Enemy Weight, Clash Combo, Knock-back Numbers, Medium Hits, KH2 Damage,
  Recoil, Guard Shape, Gummi Routes and In-game Scene Skip. With it off (the default), they're
  hidden and each keeps its default.
- `debug_menu = on` (read at start only): loads the debug tools. That means the debug menu (L1 + R1
  + Triangle), Frame Advance, the trace recorder (Select + Triangle saves a trace) and the log,
  `kh1-kh2.log`, for the categories you turn on. With it off, no log is written.

## Drive Forms

Valor, Wisdom, Limit, Master and Final are in the battle menu's **Drive** list. Each form plays
with KH2's own moves, combo and growth ability, run by KH1's engine. Anti Form is reached only by
the secret code.

- **Getting them.** With Progression > Story Unlocks off (the default), Sora owns Valor, Wisdom,
  Limit, Master and Final from the start. With it on, they open by the story:
  - The first three come after Guard Armor, Opposite Armor and the Pegasus Cup, in an order set by
    your choice at the Dive to the Heart. Sword: Valor, Limit, Wisdom. Shield: Limit, Wisdom,
    Valor. Rod: Wisdom, Valor, Limit.
  - Master comes after Anti-Sora, and Final after the second Riku fight.
  - A "New Drive Form" box says when you get one.
- **Cost** (Drive > Form Cost on the Drive gauge): Valor 3 bars, Wisdom 3, Limit 4 (or 3 with
  Drive > Limit > Cost), Master 4, Final 5. A form lasts for its form gauge, which grows with the
  form's level. With Form Cost on MP, every form costs 3 MP and then drains MP.
- **The Drive gauge** fills from Sora's hits and from KH2's Drive orbs, which enemies drop. It
  holds 9 bars, or grows from 3 with Drive Gauge Growth. In a form, Drive orbs fill the form gauge
  instead.
- **Drive Recovery and High Drive Recovery** are KH2's items, used from the menu (or in battle with
  Drive > Recovery in Battle). Progression > Drive Recovery decides whether shops sell them and
  enemies and chests give them.
- **Partners:** with Drive > Party Required at KH2, a form needs KH2's partners (see the Config
  menu).
- **Valor's and Master's second Keyblade:** the Equipment page has a row for each (V: and M:) once
  you own the form. When the slot is empty, the form uses Sora's own Keyblade. Valor needs Sora to
  hold a Keyblade (not a dream weapon or the wooden sword).
- **Wisdom** fires KH2's Wisdom shots and casts its spells.
- **Limit Form** has a Limit list in Magic's place, with KH2's limits. KH1's Trinity Limit is there only
  with Drive > Limit > Trinity Limit on.
- **Anti Form** (secret code): the Drive > Anti > Rules row sets its cost. In Anti, Sora takes 1.5×
  damage and has no guard, magic, items or growth ability. Drive orbs drain its gauge, and he
  can't talk, examine or open chests. **Anti's own look isn't built:** he looks like KH1's Sora.
- **Form levels** are kept in your save, and forms earn EXP as in KH2.

## Abilities

- **KH1's abilities stay KH1's own:** KH1's names, help, AP and effects, given on KH1's own schedule.
  The mod never takes one away or renames it.
- **KH2's abilities have their own list.** KH1's Abilities menu gets a **"KH2"** row after Sora's,
  holding them. They are: Aerial Recovery, Dodge Slash, Horizontal Slash, Negative Combo, Aerial
  Spiral, Upper Slash, Finishing Plus, Retaliating Slash, Aerial Dive, Magnet Burst, Flash Step,
  Aerial Sweep, Counterguard, Finishing Leap, Slapshot, Slide Dash, Explosion, Vicinity Break,
  Guard Break, Aerial Finish, and the support abilities Damage Drive, Form Boost, Damage Control
  and Drive Converter. Sora has them from the start, off until you equip them. They use KH2's AP
  out of Sora's AP.
- **Quick Run and Aerial Dodge** are learned (each with a "New Ability" box, off until equipped):
  Quick Run at Sora's level 12, and Aerial Dodge once Genie Jafar is beaten.
- **Bonus AP** (the main branch's rule): Sora's max AP grows by each KH2 ability's AP as he reaches
  the point where the main branch gave that ability (a level, or a boss: Sabor, Cerberus, Captain
  Hook, Genie Jafar). So the KH2 abilities don't cost you KH1 ones. There is no cap.
- **Growth abilities** (the main branch's rule, for now) level up with Sora's level and the story:
  - Quick Run: LV1, 2, 3 and MAX at levels 12, 20, 32 and 45.
  - High Jump: LV2, 3 and MAX at levels 30, 44 and 58.
  - Aerial Dodge: LV2, 3 and MAX at levels 30, 42 and 54.
  - Dodge Roll: LV2, 3 and MAX at levels 19, 37 and 60.
  - Glide: LV2 and 3 at levels 40 and 52, and MAX once Chernabog is beaten.
  - High Jump, Glide and Dodge Roll come from KH1 as usual.
  - In a Drive Form, the form's growth ability is one level above base Sora's, equipped or not:
    Valor High Jump, Wisdom Quick Run, Limit Dodge Roll, Master Aerial Dodge, Final Glide.
  - An "Ability Up" box says when one grows.
- KH1's own combo abilities (Vortex, Hurricane Blast, Ripple Drive and the rest) work in KH1's
  combo. KH2's combo uses the KH2 abilities.

## Known limits

- **Not checked in game.** Nothing on this branch has been tried in game yet. If something looks
  wrong, set `debug_menu = on`, turn on the logs you need in the debug menu's Logs page, and
  include the game folder's `kh1-kh2.log` with your report.
- **Steam only**, as above.
- **Not built yet:**
  - Anti Form's look, HP orbs, effects and ledge hang. Anti also still has the Keyblade and its
    trails.
  - Page Down to hide the HUD: not being built.
  - KH2's MP and MP Charge.
- **Experimental and off by default for now:** Gummi Routes (skipping the Gummi Ship's flights,
  which has broken story flags) and In-game Scene Skip.
- Other mods that change KH1's own code (not just its files) may conflict.

## Uninstall

The mod puts its KH2 abilities in your save's ability lists. Before removing it, take them back out:
1. In `kh1-kh2.ini`, set `abilities = remove`.
2. Start the game, load your save, and save it again at a save point.
3. Untick the mod in the OpenKH Mods Manager and Build again.

The mod also keeps a few things of its own in your save (the Drive gauge, form levels, the fourth
shortcut, Drive Recovery items). It hasn't been checked whether removing the mod tidies all of
them, so keep a backup of your save.

## For developers

The repository is Rust. `crates/kh1-hook` is the mod: it builds `kh1-kh2.dll`, which Panacea loads
into KH1. The other crates are engine-agnostic libraries and tools used to study and run KH2's rules:

```
crates/
  kh1-hook/        the mod (Windows DLL)
  kh1-data/        KH1 Final Mix's files, read from the player's own install
  kh2-formats/     readers for KH2's files (archives, BAR, 00battle, 03system, motions, models)
  kh2-data/        finds the KH2 install and builds Sora's data from it
  kh2-anim/        poses a skeleton from a motion the way KH2 does
  kh2-rig/         plays Sora's motions on his skeleton from a controller's output
  kh2-fx/          plays KH2's effect packages
  kh2-bdx/         runs KH2's BDX scripts
  kh2-magic/       KH2's spells from their own scripts
  kh2-camera/      KH2's follow and lock-on camera
  combat-core/     KH2's hit rules
  character-core/  KH2's movement and combo controller
  map-collision/   moving a body against level polygons as KH2 does
  kh2-combat-cli/  `kh2-combat`: dump tables and run checks
  kh2-bevy/        a Bevy playground
docs/              plain-language notes on how KH1 and KH2 work, and what was built
AGENTS.md          every version's change, newest first
```

How the branch works: `docs/testing-merge.md`, `docs/design-philosophy-instructions.md` and
`docs/engineering-rules.md`. Testers' requests: `docs/tester-requests.md`. Run the tests with
`cargo test`. They need no game data.

Rules this repository follows:
- No game assets, dumps or decompiled code are committed. A whitelist `.gitignore` enforces it.
- Single-player, offline, for a copy you own. No DRM or anti-cheat is touched.

## Credits and licence

`CREDITS.md` and `NOTICE` list the work this builds on (OpenKH's documentation and code, gaithern's
KH-1FM-AP-LUA, the KH1FM Randomizer and others) and their licences. Most crates are MIT OR Apache-2.0
(`LICENSE-MIT`, `LICENSE-APACHE`). `kh2-formats`, `kh2-data`, `kh2-fx` and `kh1-data` are
Apache-2.0: they contain ports of OpenKH code, or build on it.

Kingdom Hearts is a trademark of Disney and Square Enix. This project is not affiliated with or
endorsed by them, and contains no game assets or code.
