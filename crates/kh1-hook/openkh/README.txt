KH2 Combat for KH1
==================

Plays Kingdom Hearts Final Mix with Kingdom Hearts II's combat and movement:
KH2's combos and finishers, the KH2 combo abilities (Upper Slash, Explosion,
Magnet Burst, Aerial Finish, Vicinity Break, Flash Step and more), Quick Run,
Aerial Dodge, Glide, Dodge Roll, Guard, Aerial Recovery, KH2's field camera and
a fourth shortcut slot. Sora's KH2 animations, sounds and effects are read
straight from your own KH2 install; nothing from either game is included here.


WHAT YOU NEED
-------------
1. KINGDOM HEARTS HD 1.5+2.5 ReMIX for PC, the Epic Games Store or the Steam
   release, with both KH Final Mix and KH II Final Mix in the same install
   (they are, by default). The mod checks which one it is running in; on any
   other version it stays switched off and changes nothing.
2. OpenKH Mods Manager, set up for KH 1.5+2.5 with Panacea installed:
   https://github.com/OpenKH/OpenKh/releases
   (On first run, its setup wizard asks which store your game is from and for
   your game folder, and offers to install Panacea: say yes. Panacea is what
   loads this mod into the game. Steam's folder is usually
   ...\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5 ReMIX-)


INSTALL
-------
1. Back up your KH1 save (recommended). Saves are in:
   Documents\KINGDOM HEARTS HD 1.5+2.5 ReMIX\Epic Games Store\<number>\
   or, for Steam, Documents\KINGDOM HEARTS HD 1.5+2.5 ReMIX\Steam\<number>\
2. Open the OpenKH Mods Manager and pick "Kingdom Hearts Final Mix" (KH1) as
   the game.
3. Mods > Install a New Mod, and enter PM2CRAZY/KH2-Combat-In-KH1-Mod (a small
   repository holding only the latest build, so it downloads in seconds).
   Or, with a zip: "Select and install Mod Archive" and choose the zip file
   (don't unzip it).
4. Tick "KH2 Combat for KH1" in the mod list.
5. Build > "Build and Run" (or "Build Only", then start the game normally).
6. Load a save. That's it.

The first launch makes a settings file, kh1-kh2.ini, in your game folder (the
one with "KINGDOM HEARTS FINAL MIX.exe"), and a log, kh1-kh2.log. Settings are
read when the game starts.

UPDATING: OpenKH's Mods Manager updates a mod installed from
PM2CRAZY/KH2-Combat-In-KH1-Mod (or install a new zip the same way), then Build
again (an old build left in OpenKH's output folder keeps loading otherwise). To check which
version is running, open Config in the game and move to the "Camera Style"
row: its help box says "KH2 Combat version s154" (or whichever you have). The
first line of kh1-kh2.log says it too.


CONTROLS (what changes)
-----------------------
Attack ............ KH2's combos, finishers and combo abilities
Jump in the air ... Aerial Dodge (once you have it)
Square ............ Dodge Roll / Quick Run while moving, Guard while standing,
                    Glide in the air (set "glide = jump" to glide by holding jump)
L1 + cancel ....... the fourth shortcut (set it in the Customize menu)
Right stick ....... KH2's camera (turn and zoom); R3 resets it
D-pad left ........ at the battle menu: the second page (Attack, Magic, Items,
                    Drive). Drive > Wisdom Form costs 3 MP, then drains
                    the yellow part of the MP gauge bit by bit (a blue bar
                    breaks into yellow when it runs out; half as fast out
                    of battle); out of MP, Sora reverts. In the form the same entry is Revert, and
                    Fire, Blizzard, Thunder and Cure cast KH2's Wisdom versions.
Magic, items, summons and specials stay KH1's own.
The Config menu has a "Camera Style" row to switch between KH1's and KH2's
camera.


HOW ABILITIES WORK
------------------
- KH1 abilities that have a KH2 version become it when equipped (Vortex is
  Flash Step, Hurricane Blast is Explosion, Ripple Drive is Vicinity Break,
  Stun Impact is Guard Break, Gravity Break is Aerial Finish, Sliding Dash is
  Slide Dash, Counterattack is Counterguard, Blitz is Finishing Leap...).
- KH2-only abilities are added to your Ability menu as you level up or beat
  bosses (a "New Ability" box tells you), with their own AP: your max AP
  grows by the same amount, so they never cost you KH1 abilities.
- High Jump, Glide, Quick Run, Aerial Dodge and Dodge Roll level up with Sora.
  Glide reaches MAX for beating Chernabog (Superglide is folded into it).


SETTINGS (kh1-kh2.ini)
----------------------
Each line in the file explains itself. The ones you are most likely to want:
  camera = kh2 / kh1          which camera
  glide = square / jump       which button glides
  damage_scale = 1.0          multiplies the damage of your KH2 attacks
  magic_scale = 1.0           extra multiplier for Explosion / Magnet Burst
  kh2_effects = off           turns KH2's visual effects off
  kh2_sounds = off            turns KH2's sound effects off
  shortcuts = 3               back to KH1's three shortcuts
  drive = off                 no second menu page and no Wisdom Form
  kh2_hair = off              hair, hood and keychain swing with the older
                              physics instead of KH2's own animation of them
  physics = off               hair, hood and keychain stay still in KH2's moves


UNINSTALL
---------
The mod adds its KH2 abilities to your save's ability list. Before removing
the mod, take them back out so the save is plain KH1 again:
1. In kh1-kh2.ini set:  abilities = remove
2. Start the game, load your save, and save it again at a save point.
3. Untick the mod in the OpenKH Mods Manager and Build again.


KNOWN LIMITS
------------
- Work in progress: if something looks wrong, the game folder's kh1-kh2.log
  says what the mod was doing; please include it with any bug report.
- Other mods that change KH1's own code (not just its files) may conflict.


CREDITS AND LICENCE
-------------------
MIT or Apache-2.0, your choice (LICENSE-MIT, LICENSE-APACHE). NOTICE and
CREDITS.md list the work this builds on (OpenKH's documentation, gaithern's
KH-1FM-AP-LUA and others) and their licences.
Kingdom Hearts is a trademark of Disney and Square Enix. This project is not
affiliated with or endorsed by them, and contains no game assets or code.
