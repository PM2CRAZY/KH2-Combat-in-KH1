# testing-merge: what to test before release

Everything on this branch that hasn't been seen in game yet (all of it, as of the version at the bottom). Each line
names the version that built it. Play with logging on (the debug menu or the ini's log categories) and keep
`kh1-kh2.log` for anything that looks wrong: most of these versions log what they did.

Logs worth turning on: General, Menus, Combat, Forms, Magic, Sound, Effects, Animation, Loading & Cache.

## 0. Start-up and saves

- [ ] A clean install from `mod.yml` + `kh1-kh2.dll` starts; the Config button's help shows the branch's version (`tm...`). (README)
- [ ] First start builds the caches (effects, sounds, motion slots, Anti's textures) and the second start loads them quickly. (tm4, tm20, tm22, tm23, tm27)
- [ ] An old save from testing and one from main both load; abilities, Drive levels and Keyblades look right. (tm7, tm8)
- [ ] Save menu > return to title, Continue: no stuck loading screen; also soft reset and game over to the title. (t1079 didn't fix it; this round)
- [ ] Load one save, go to title, load another: no false "Ability Up" box. (t1084)

## 1. Config menu

- [ ] Every list's help line fits on screen (QoL, Progression, Drive, Tweaks especially). (t1078)
- [ ] KH2 is on the right on every row: Drive > Limit's Cost and Ledges. (t1077)
- [ ] Command Menu: the row is "2nd Page", Off / On. (t1087)
- [ ] With `experimental = off`: QoL has no Gummi Routes or In-game Scene Skip; with `experimental = on` they're back, off. (t1086)
- [ ] Tweaks > Walls: KH1 / Clink / Pass; untouched, it follows Recoil (Clink with Recoil KH2). (tm9, tm13)
- [ ] Progression > Reflect by Story: on by default on a new ini. (tm6, tm15)
- [ ] Drive > Anti > Rules: KH2 / Balance. (tm10)
- [ ] The Drive list's "DRIVES" title sits centred in its tab, white. (main s304, this round)

## 2. Abilities, AP and growth

- [ ] Max AP grows by main's schedule: +2 at level 4, Dodge Slash at 10, Quick Run at 12, Damage Drive 14, ... and after Sabor (Upper Slash), Cerberus (Aerial Dive), Captain Hook (Magnet Burst), Genie Jafar (Aerial Dodge). No cap. (tm12)
- [ ] Quick Run appears at level 12 and Aerial Dodge after Genie Jafar, off, with a "New Ability" box. (tm8)
- [ ] Growth levels rise with Sora's level (Quick Run 12/20/32/45, High Jump 30/44/58, Aerial Dodge 30/42/54, Dodge Roll 19/37/60, Glide 40/52, MAX after Chernabog); each form's growth is one above, even unequipped. (tm8)
- [ ] High Jump and Glide only come where KH1 gives them. (tm8 / t1085)

## 3. Drive Forms (all)

- [ ] Each form: enter, fight, revert; a room change in a form (effects come back? see the open question on t-branch formfx); death in a form.
- [ ] No guard and no dodge roll in Master and Final, at both Dodge Roll styles, keyboard and pad. (this round)
- [ ] Wisdom and Final: no footsteps on the ground; the other forms and base Sora still have theirs. (t1088)
- [ ] Final's Blizzard finisher casts its full set; its ground and air finishers show their effects. (t1093, t1094)
- [ ] Limit Form: L1 shows its limits; your shortcuts come back after the form, a save in the form keeps your own. (t1080)
- [ ] The Coliseum: win a round in a form with Donald and Goofy (no soft lock), next rounds, a cup end. (t1091)
- [ ] The second Keyblade has no shadow (known, not fixed: needs the exe).

## 4. Anti Form

- [ ] The secret code in battle with 3+ bars (and Donald and Goofy with Party Required on) starts Anti. (tm11)
- [ ] Rules KH2: needs 3 bars, takes 9, Revert greyed in battle; Balance: free, no Revert. (tm10)
- [ ] ×1.5 damage taken; no magic, items or summons (greyed); no talk / examine / open. (tm10, tm14, tm17)
- [ ] Square: Anti Glide on the ground, Side Spin mid-combo. (tm10)
- [ ] No Keyblade and no trail; the Keyblade is back after revert. (tm18)
- [ ] HP orbs aren't picked up in Anti. (tm19)
- [ ] Its KH2 effects and sounds play. (tm20, tm22)
- [ ] Ledge: grab, hang (by the foot), climb; a blocked top. (tm23)
- [ ] The look: KH1 Anti-Sora's textures, solid yellow blinking eyes; also after a room change and on the pause menu's Sora. First entry takes about 3 s to build. (tm24–tm27)

## 5. Combat

- [ ] Walls at each Walls value; a swing that touches a wall still hits an enemy after (KH1 value). (tm9)
- [ ] A KH2 combo toward a room exit doesn't change rooms; walking does. Crates don't push mid-combo. (tm3)
- [ ] Counterguard, Flash Step and Retaliating Slash show their KH2 effects. (tm4)
- [ ] Magnet Burst's hum stops with its effect (finish it, cancel it). (t1090)
- [ ] Command Lock at KH1 behaves as vanilla. (t1092: logs only)
- [ ] The Dream Shield doesn't stretch (combos, item use, a cast). (tm5)

## 6. Items and Reflect

- [ ] Drive Recovery / High Drive Recovery in battle (Recovery in Battle on): named right, the target list, adds +3 / +9 bars, used once. (t1082, t1083, this round)
- [ ] Reflect by Story: Reflect after the first of Pot Centipede / Hercules Cup / Ice Titan, Reflera, Reflega, each with its box. (tm6)

## 7. Camera and QoL

- [ ] KH2 camera with KH1's camera up/down on Reverse: zoom and first-person pitch flip. (tm1)
- [ ] Gummi skip (experimental): skip a route and fly the same one from the same save; story steps match. (t1089, needs the route-results write still)
- [ ] In-game Scene Skip (experimental) still works when on. (t1086)

## Known not built

- Epic Games Store support; the Page Down HUD (the owner's choice).
- The second Keyblade's shadow (needs the exe's bytes).
- Form effects after a room change on testing's question (arrival pose or not).
