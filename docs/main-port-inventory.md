aligned with main s339 / testing t930

# Main → testing port inventory

Phase 1 of `docs/main-port-instructions.md` (section 3). Docs only: no version bump. Written 2026-10-29.

**How this was built.** Main read from `origin/main` (s339, commit `4adb946`, the same version as the local
`KH2-Combat-in-KH1-main` copy), testing from `origin/testing` (t923). `origin/main` is not newer than the
local copy. Sources read: every module in main's `crates/kh1-hook/src` (and `win/`) against testing's, every
key in both `settings.rs` files and the places they are read, main's Config rows (`win/config.rs`), every
main doc, the files main has in the other crates, main's `AGENTS.md` (s60–s339; every "the user: ..." quote in
the rows below is from there), main's `README.md`, `openkh/README.txt` and `docs/kh1-port.md`.
The merge base is main **s260**: everything main did in s261–s339 is main-only; older main features were
inherited by testing and many were rebuilt or deleted since (the KH2 controller, t901).

**Tags.** **Checked (code)** = read in the branch's code (file:line). **Assumption** = inferred, not checked.
Main's "Checked" tags are Epic and controller-era, so they count as **Assumption** here.

## The owner's rules for this port (chat, 2026-10-29)

- **Testing's layout is the base.** Every main feature is rebuilt inside testing's own systems (Drive
  Forms, magic calculations, abilities, ...). Where main and testing differ, testing's system wins and the
  main feature is fitted to it. The owner: "generally everything should go to how testing is layered out".
- **Main's ini is deprecated.** No ini carry-over and no ini converter. Ini keys that do nothing on testing
  are removed from testing (its own version). The owner: "any ini setting that does nothing in the testing
  branch should be removed".
- **Abilities follow testing's rule** (KH2's abilities separate from KH1's, the KH2 list, vanilla KH1 kept).
  The owner: "abilities were overwriting vanilla abilities on the main branch but is now separate so we go
  with testings rule". Main's support abilities get new ids in testing's KH2 list.
- **A main save is reconfigured on load** so every ability matches testing's layout. The owner: "the save
  should reconfigure all the abilities to mach the testing branch".
- **Drive Forms cost MP or the Drive gauge, by a toggle** in a Drive submenu of the Config menu. The owner:
  "we'll have a toggle to have drive forms cost mp or drive meter (will go in the drive config submenu)".
  Testing has no Drive submenu yet (only Command Menu > Drive Page): it is built with the gauge.
- **Cure follows KH2's rules, later.** The owner: "cure should follow kh2's rules but leave it for now and
  note it off". Noted in row 31; no work now.
- **The owner's own balance and progression rules from main go behind Config toggles (on / off).** The
  owner: "have it be a config and make it a quick balance changes config with it on or off, so we can
  preserve the changes but work on it later, same goes for the progression changes in the main branch".
  Balance: Limit 3 bars, KH1's limits in Limit Form, Anti free with no revert, item drop rates and chests,
  the Dream Sword default. Progression: the story unlocks of forms (row 8), the six-fight gauge maximum
  (row 1), the support abilities' learn levels (row 5). With a toggle off, KH2's rule (or testing's) plays.
- **The secret code gives Anti Form** (main s272). The owner: "(a)".
- **Every port checks the Steam C decomp for what KH2 has.** The owner: "when porting stuff over to the new
  system to check the c decomp to make sure we aren't missing anything from kh2". Section 2b of the
  instructions, for every row, before its plan is final.

**Decomp check (section 2b): not started for any row.** This round read code and docs only; the Steam C
decomps were not opened. Each row's decomp check is done in its research round. Steam addresses that
testing's docs already found are noted where they exist.

**Deprecated dependencies, named once:** [CTRL] KH2's controller / `takeover`'s driver (deleted t901);
[SWEEP] our own hit sweeps (`combat::Strikes`, `kh2_rig::MotionHits`); [SCRIPT] our own script runtime;
[RENDER] our own renderer / pose pass; [STEER] per-frame steering hooks; [DECODE] KH2 assets decoded at
every start; [EPIC] Epic RVAs mapped through `steam_map.in`, not re-found on Steam.

## Counts

| Status | Rows |
|---|---|
| Ported (native) | 16 |
| Ported (running along) | 0 |
| Partly | 7 |
| Missing | 18 |
| Superseded | 10 |
| Obsolete | 10 (each needs an in-game confirm where marked) |

Plus two rows that aren't statuses: reaction commands (main's KH1 mod never had them) and the merge-gate
items (Epic, README / packaging).

---

## A. Drive Forms cluster

Common ground on testing (Checked (code)): `form::Kind` is Valor, Wisdom, Final (`win/form.rs:79-83`); the
Drive list holds Valor and Wisdom (`win/drive.rs:32`); only Valor and Wisdom can be entered
(`native_capable` `form.rs:125`; Final refused on Steam, `form.rs:363`, t873). A form costs `COST_MP = 3` and
drains one yellow MP unit every 3 battle frames; it reverts only on a KO, the Revert command or 0 MP
(`form.rs:667-708`). Form levels and EXP are KH2's fmlv rule, 7 levels (`formlevel.rs`, `win/formexp.rs`,
t700), stored at save `+0x882` / `+0x884` / `+0x886`. `character-core/src/drive.rs` (KH2's gauge rules) is
identical on both branches and unused by testing's `kh1-hook`.

| # | System | On main | What the player sees | Owner's decisions (main) | Testing now | Status | Conflicts | Plan (one version each) |
|---|---|---|---|---|---|---|---|---|
| 1 | Drive gauge data | `drivegauge.rs`, `win/gauge.rs`; s318, s323, s325, s326, s331 | Bars of 100 points. Max 3, +1 each for Sabor, Riku 1, Chernabog, Sephiroth, Kurt Zisa, the Unknown (cap 9). Costs Valor 3, Wisdom 3, Limit 3, Master 4, Final 5, Anti 9 (needs 3). Fills from hits (KH2 ATKP drive value; KH1 hits 3 physical / 5 spell), damage taken (Damage Drive only), orbs (1 / 3). In a form the gauge is the form gauge `(level + 3) × 600` frames, 1 a frame in battle, 0.5 outside. Full by default. MP cost and drain removed. | s318 "lets incorporate the drive gauge"; s323 "don't show the drive gauge until the first drive form is unlocked. By default, the drive gauge should be full."; s325 the six-fight table, "when in a form, have the drive capacity be raised to whatever level the form is"; s326 Chernabog instead of beating the game; s331 "Make limit form only cost 3 bars to enter" | No gauge; forms on MP. Planned as step 1 in `docs/kh1-drive-forms-native.md` §5 (KH1 hit contact `0x1402BF330`, HP change `0x1402A4920` noted on Steam). | Missing | **Settled (owner):** a Config toggle in a new Drive submenu picks MP or the Drive gauge as the cost. The form gauge uses testing's KH2 form levels 1..7 (KH2's `(level + 3) × 600`, 1:1); main's growth-level rule (s325) is dropped. Still open: the max table and Limit's 3 bars are the owner's, not KH2's. | Gauge data in the save + KH2's add rules + the hit add at KH1's contact; then the Drive submenu and its cost toggle (MP / Drive gauge); then the form gauge and drain. |
| 2 | Drive gauge HUD | `win/gaugehud.rs`, `win/kh2hud.rs`, kh2-formats `seqd.rs` `dds.rs` `png.rs`; s319–s324, s326; ini `drive_gauge_art`, `drive_gauge`, `drive_gauge_scale`, `hide_hp_label`, `drive_gauge_debug` | KH2's DRIVE bar, MAX plate, number, FORM bar in a form, left of / below Sora's portrait inside KH1's HP/MP task with KH1's sprite calls; optional KH2 `gaug` art from `zz0field.2dd`; only while the player controls Sora; KH1's "MP HP" label hidden. | s320 "use ghidra if you have to on KH2 to get the exact graphics, the animation while the gauge is charging, when its being drained, and all the animations"; s321 no rise-in animation; s322 "This should be the right size and positioning"; s324 bar outlines restored; s326 "only showing when the character is actively controlling sora" | Nothing. KH1's HUD task noted on Steam `0x14026DB00` (field meanings Assumption). | Missing | None. | KH1 pieces on KH1's HUD; then KH2's art built once into the mod folder (main decoded it every start [DECODE]); visibility from a KH1 signal (main used `takeover::script_has_sora` [CTRL]). |
| 3 | Drive orbs as KH1 prize types | `win/prize.rs`, `win/prizeart.rs`, kh2-formats `prize.rs`; s318, s328 | Prize types 7 / 8 in a copied KH1 prize row table; KH1's spawner, pickup, scatter and blink run them. Drop weights after KH1's own drop. KH2's orb sprite from `obj/PRIZE.mdlx` through KH1's texture lookup. | s318 "I want them to run through KH1's engine, not an overlay"; s328 "if possible, let's try to get the texture from KH2" | Nothing. Steam addresses already in `kh1-drive-forms-native.md` §3 (make `0x1402AB2A0`, drop `0x1402ABC40`, pickup `0x1402AC080`, rows `0x140528BE0`, Checked (decomp) there; they differ from main's). | Missing | Drop weights partly ours ("six small and two big"), not KH2's `przt`: check. | Prize types on KH1's prize code; then the art built once [DECODE on main]. |
| 4 | Drive Recovery / High Drive Recovery | `driveitems.rs`, `win/items.rs`; s326, s327, s333 | KH1's unused item ids `0x91` / `0x92` as KH2's items (+3 / +9 bars), pause-menu use only, bought 80 / 120; in item shops with a Potion (Traverse Town `tw10`, Agrabah `al19`) once a form is open; enemy drops (3% / 0.5%, MP orbs 5% / 0.75%, 20+ prizes 15% / 4%, × Lucky Strike); 17 chests patched (12 / 5). | s326 "have drive recovery items exist and be sold at the item shop in traverse town"; s333 "more available throughout the game (mob drops, chests, etc)" | Nothing (no 0x91 / 0x92 anywhere). Testing's `win/chest.rs` hooks KH1's chest Open (`chest.rs:211-225`). | Missing | Rates, prices, chests and shops are the owner's design (KH2 never sells them). Chest table patching must coexist with `chest.rs` (Assumption: no clash). Ids must stay 0x91 / 0x92 for main saves. | Item records + shops; then drops and chests. |
| 5 | KH2 support abilities: Damage Drive, Form Boost, Damage Control, Drive Converter | `support.rs`, `growth.rs` `MENU` 78..81 + `SUPPORT_LEVELS`, `win/hurt.rs`, `win/gauge.rs`, `win/prize.rs`; s335, s337 | Learned at Sora's level 14 / 24 / 38 / 35, AP 3 / 5 / 5 / 5, KH2 names and help. Damage Drive damage / max HP × 50 a copy outside a form; Form Boost drain × 0.8 a copy; Damage Control halves damage at ≤ 25% HP; Drive Converter munny × 0.5, the rest orbs. Berserk Charge and Drive Boost left out (no MP Charge). | s335 the list, Berserk Charge and Drive Boost left out, "At set levels"; s337 "make drive converter come at level 35" | Nothing. **Ids 78..81 are Flash Step, Aerial Sweep, Counterguard, Finishing Leap on testing** (`growth.rs:356-359`, Checked (code)). Form Boost known only as an fmlv reward that "waits for the Drive gauge" (`growth.rs:583`). | Missing | **Settled (owner):** testing's ability rule; the four get new ids in testing's KH2 list, and a main save's 78..81 are reconfigured on load (section "Save"). Still open: testing's t718 vanilla-progression rule ("just give the player all the custom abilities") vs main's set levels. Drive Boost / Berserk Charge wait on the open KH2 MP / MP Charge thread. | After the gauge and orbs: one version for the four abilities (new ids). |
| 6 | Forced reverts (bars refilled, partners back), back to Sora in a new world | `win/form.rs` `revert(forced)`, `gauge::refill_bars`, `vanish::force_back`; s275, s308 timings, s338 | A revert outside the player's control (KO, new world, scenes, battle end, Coliseum arena) refills every bar and brings partners fully back; Revert command and gauge-out don't. | s275 "when leaving a world in a drive form ... he should be automatically reverted"; s338 "make sure the party members are fully visible, and fully refill sora's drive gauge" | Reverts only on KO / Revert / 0 MP (`form.rs:674-707`). No world, scene or battle-end revert (Assumption: none elsewhere). | Missing | None. Main's scene detection was [CTRL] (`takeover::scene_frame`, `script_has_sora`): KH1 signals needed. | One version, after rows 1 and 11. |
| 7 | Drive Forms across Olympus Coliseum rounds | `win/cup.rs`, `win/vanish.rs`; s315, s332, s334 | Arena entry reverts; a won round keeps the form (no drain in the victory scene); partners restored before KH1's victory wait; none left invisible after a cup. | s332 "if you enter the cup in a drive form you should be reverted before the first battle, but if you defeat a round in a drive form it should carry over into the next round"; s315, s334 bug reports | Nothing for forms. Testing's `ingameskip` rewires cup event function 512 in the same flow. | Missing | Must be checked together with `ingameskip`'s 512 hook. Main's op 103 fix existed only because [CTRL]'s vanish cleared 0x8000. | One version, after rows 6 and 11. |
| 8 | Form unlocks by the story; "New Drive Form" box | `story.rs` (pure), `win/menu.rs` `form_box`; s279, s307, s308 | First form after Guard Armor, second after Opposite Armor, third after the Pegasus Cup; order by the Dive choice (Sword: Valor, Limit, Wisdom; Shield: Limit, Wisdom, Valor; Rod: Wisdom, Valor, Limit); Master after Anti-Sora, Final after Riku 2. Read-only from the save (TT `+0x1504`, journal `+0x16E3`, cups `+0xF36..`, Dive choice `+0x48C`). | s307 the owner's table, "dont use the archipelago work just make it yourself"; s308 "the game should tell you when you get a drive form and reflect" | No gating: both forms usable whenever MP > 3 (`drive.rs:232-245`, `form.rs:316-323`). | Ported (native) (t925–t930; not checked in game) | Behind Progression > Story Unlocks (off by default). Limit and Master are stored and wait for their forms. | Done: t925 owned byte, t926 Story Unlocks, t928 Drive list, t929 box, t930 Valor's slot. |
| 9 | Reflect levels by the story; "New Magic: Reflect" box | `story.rs`, `win/reflect.rs::level`, `win/menu.rs` `reflect_box`; s307, s308 | Reflect / Reflera / Reflega by how many of Pot Centipede, Hercules Cup, Ice Titan are won. | s307 (as row 8) | Reflect follows KH1's Aero level (`win/reflect.rs:124-136`), main's pre-s307 rule. | Missing | Testing's rule (Aero level) stands under the owner's "testing's layout is the base"; main's boss rule only if the owner asks. | With row 8, only if asked. |
| 10 | Forms need their partners | `partners.rs`, `form::field_party`; s311 | Valor needs Goofy, Wisdom Donald, Master and Final both, Limit none; checked on entry (field party id `+0x4C`, KO bits). | s311 "make drive forms require their specific party members to cast" | Nothing (`form.rs:316-345`). | Missing | None (KH2's rule; decomp check pending). | One version. |
| 11 | Partners vanish during the form (and their HUD entry) | `win/vanish.rs`; s312, s315, s332, s334, s338 | The form's partner disappears (actor flags, HUD slot fade `0x140266CC0` Epic) and comes back on revert, events, room change, game over. | s312 "the party member associated with the drive from also disappears (along with their hud icon)" | Nothing. | Missing | Main called `takeover::script_has_sora` [CTRL] four times; [EPIC]. | One version. |
| 12 | Limit Form | `win/limits.rs`, `kh2-magic` limit tests, `win/form.rs`, `win/tint.rs`; s287–s299, s314, s331, s336 | KH2's Limit moves (Zantetsuken, Ripple Drive, Hurricane Period, guard, Zantetsu Counter, Dodge Roll); black jumpsuit and jacket, yellow belt, black shoes; Magic becomes a Limit menu of KH1's five limits (run by KH1, with or without the abilities), KH2's own limits built and switched off (s297); KH2 FM's shortcut table; 3 bars; limits heal per hit with KH2's `hp_drain` and work out of battle; no revert mid-limit. | s291 "KH1 limits that are equipped shouldn't even prompt in any drive form"; s293, s296 the outfit; s297 "comment the KH2 limits, and for the moment put in the equivalent KH1 limits"; s299 Limit menu, not Triangle prompts, KH2's default shortcuts; s331 3 bars; s336 "he should get health back ... use the limits whenever, even when out of combat" | None (`form.rs:79-83`; `docs/kh1-kh2-limits.md` §1). The `testers/drive-forms/` thread still says Limit runs on the old controller: stale. | Missing | 3 bars and KH1 limits instead of KH2's are owner overrides of KH2 (Config toggle or keep: ask). Main ran on [CTRL], [SWEEP], [SCRIPT]. **Rule 2a: testing has no native Limit Form to build on; the form's moveset has to be baked onto KH1's states first, the way Valor and Wisdom were.** | Moveset on KH1's states; look; Limit menu of KH1's limits; heal per hit and out of battle. |
| 13 | Master Form | `win/form.rs`, `win/equip.rs` M: row; s300, s301 | KH2's Master moves and combos, Aerial Dodge one above Sora's, Wisdom's magic rules, floating second Keyblade, M: equipment row (save `+0x48B`), yellow look; its KH1 Gravity travels with the cast's root. | s300 "take the rules and procedures we've done for wisdom form and valor form, and lets make master form"; s301 KH1 magic "doesn't actually move sora at all" | None. | Missing | Save byte `+0x48B` clash (section "Save"). Main on [CTRL] / [RENDER]. | After Limit, on KH1's states. |
| 14 | Anti Form (and its look) | `win/form.rs`, `win/tint.rs`, `kh1-data/src/texmap.rs`; s272–s292, s285 | By the secret code; free (later 9 bars, needs 3), no revert, 1.5× damage taken, no guard, no growth; magic, items, summons, limits, Drive refused; Anti Glide and Side Spin; ledge hang by the foot; no Keyblade or trails; KH1 Anti-Sora's textures on Sora's HD textures, solid yellow eyes RGB (253, 230, 95). | s272 "I want 1:1 combat parity. I dont want it to cost any MP to enter, and I dont want sora to be able to revert out of it"; s273, s277, s280 the ledge hang; s283 no guard; s285 "Antiform should take half of your MP"; s290 "copying the exact textures of KH1 anti-sora onto sora's anti form model"; s292 the eyes | None. Testing's secret code still gives Final (`form.rs:204-227`); no `texmap.rs`; `tint::Look` is Wisdom and Valor. | Missing | **Settled (owner):** the secret code gives Anti. Anti free / no revert goes behind the balance toggle (KH2's Anti counter and cost with it off; the counter wasn't used by main). Texture map built every start (~3 s) [DECODE]. | After Master, on KH1's states. |
| 15 | Final Form | `win/form.rs`, `kh2-rig/tests/final_form.rs`, `kh2-anim/examples/final_blades.rs`; s231–s283 | Secret code (s231), then in the Drive list for 3 MP with the code doing nothing (s271); white outfit, black belt; Wisdom-style magic; combo effects on the floating blades; F: equipment row (save `+0x489`). | s271 "put it into the drive menu, and have the cheat code do nothing for the moment"; s283 "add a section in the equipment menu for the second keyblade in final form" | `Kind::Final` exists with the code but is refused on Steam (`form.rs:125,363`, t873); second blade fixed at the Kingdom Key (`valorstats.rs:64`); no Final tint. Owner on testing: "we're not going to touch final form unless I change my mind" (`kh1-drive-forms-native.md` §10), "we're going to transfer it over when ready" (t873). | Partly | On hold by the owner. Main s271 / s272 vs testing's code. | Owner's call. |
| 16 | Forms' growth abilities | s270, s301 | One level above base Sora's, even if unequipped. | s270 "always be one level higher than the level unlocked by base sora" | KH2's fmlv: growth by form level (Lv1/3/5/7 → LV1/2/3/MAX), `form.rs:446-457`, the tester's answer 6 (b). | Superseded | Testing's KH2 1:1 rule. | — |
| 17 | Drive heals | s230 (before the merge base) | HP full on entry. | — | `heal_full` (`form.rs:275-287`) in `native_start`. | Ported (native) | KH2 also refills MP on entry (the drive-forms thread, m1): testing refills HP only. Check in the decomp. | — |
| 18 | Talking / examining in forms | s263 | Sora can talk and open / examine in a form. | s263 "let sora be allowed to talk to people and interact with objects in drive forms" | Valor's refusal (`combat.rs:585`) lets through kinds 0/1/2/item and `NOT_SORAS` only. | Missing (Assumption: not checked in game; Wisdom unaffected) | None. | Small version with the forms work. |
| 19 | No limits in any form | s278, s291 | No KH1 limit offered or usable in any form. | s278 "sora should not be able to use limits in ANY drive form including anti form" | Valor refuses them (`combat.rs:579-592`); Wisdom doesn't. | Partly | None. | Small version. |
| 20 | Second-Keyblade equipment rows and rules | `win/equip.rs`, `win/equipmenu.rs`; s212, s220, s262, s283, s286, s306 | V: (Valor) row; F: and M: rows; the compare box shows the form's stats (s262); a slotted Keyblade counts as in use, also for the shop's sell list (s286); the Dream Sword is Valor's default (s306). | s262 "it doesnt accurately display its stats"; s286 "you able to have the same secondary keyblade for multiple forms"; s306 "i want the Dream Sword to be in the second keyblade slot by default" | V: row at `+0x488` (`equip.rs:52`, `equipmenu.rs:335-582`, `valorstats.rs`). No compare box hook, no in-use count hook (`0x1402E6330` only read, `equip.rs:108`), no default; testing treats the Dream Sword as not a Keyblade (`equip.rs:46`). | Partly | Dream Sword default vs testing's Keyblade-only rule. | Compare box; in-use count; default (after an answer); F: / M: with their forms. |
| 21 | Grey screen tint during a Drive change | s332 | KH2's grey tint during the change. | (found by main as missing) | Not found. | Missing | None. | One version (as a KH1 effect or fade: decomp check first). |
| 22 | Revert limb glows end | s332 | The revert's glows stop after 60 frames. | — | `kh2fx::owner_gone` ends the owner's effects at the form change (`kh2fx.rs:76-80`). | Ported (native) (Assumption: covers the revert glows; not checked in game) | — | Confirm in game. |
| 23 | MP costs and drains s268 / s269 / s285 / s310 | — | 1 MP forms, 2 extra MP bars, half MP for Anti, slower drain. | s268, s269, s285, s310 | 3 MP, drain every 3 frames. | Obsolete | Replaced on main itself by the gauge (s318). If testing keeps MP until row 1, nothing to port. | — |
| 24 | Drive list without costs | s324 | No numbers in the Drive list. | s324 "remove the numbers in the drive menu" | Testing's list never drew them (`drive.rs:232-245`). | Ported (native) | — | — |
| 25 | Stuck in a form after a boss | s308 | — | s308 bug report | The cause was [CTRL] reading KH1's script hold; KH1's states always have him on testing. | Obsolete | The `scene_revert` timings go with row 6. | Confirm in game (Opposite Armor in Wisdom). |

## B. Everything else

| # | System | On main | What the player sees | Owner's decisions (main) | Testing now | Status | Conflicts / deprecated | Plan |
|---|---|---|---|---|---|---|---|---|
| 26 | HUD hide key | `win/hud.rs`, `keys.rs`, `win/sys.rs::game_focused`, `docs/kh1-hud-hide.md`; ini `hud_toggle_key`; s312 | Page Down (configurable) hides the whole field HUD; press again to show. | s312 "can you also make it so that pg dn hides the hud?" | Nothing. | Missing | Main hooks KH1's immediate draw (`0x1400FBBA0` Epic) on every HUD draw call every frame [STEER-like cost]; [EPIC]. Needs a cheaper native point (main's doc: KH1 has no single HUD flag). | One version after a design round. |
| 27 | Music mute | `win/music.rs`, `docs/kh1-music-mute.md`; ini `music_mute` (lowest / on / off); s311 | Lowest Music step silent, or music always silent, or vanilla. | s311 "make it where music can be fully muted in the settings" | `win/volmute.rs` (t915, t916): every slider's lowest step is silence, data and code KH1 runs, nothing per frame. | Superseded | Main's `on` (always silent) has no equivalent: ask. | — |
| 28 | A jump that backs out of a submenu doesn't jump | `menujump.rs`, `win/menujump.rs`; s317 | Closing Magic / Items / Summon / Drive with jump doesn't also jump. | s317 "back out of the menu first before allowing jumps" | KH1's own menu step and jump use the press (`win/shortcuts.rs:712-718`, `takeover.rs:545-558`). | Obsolete (confirm in game: open Magic, press jump) | The bug came from [CTRL] reading the raw pad. | — |
| 29 | No-damage time after a hit | `mercy.rs`, `win/hurt.rs`; s302 | One hit a pass, no damage during KH2's reaction and 30 frames after. | s302 (a player: Large Body "nuked" Sora) | KH1 plays its own reaction and sets its own no-damage timer; KH2's invulnerability goes on KH1's timer (t858–t868); `hitreact.rs`. | Obsolete | Existed only because KH2's reactions replaced KH1's. | — |
| 30 | Water, pits, swimming, midair climbs | `docs/kh1-water-pits.md`, `takeover.rs`, `gates.rs`; s311 | No walking on the sea floor, no falling through the Bottomless Hall, no stuck swim / climb pose. | s311 (a player's report) | KH1-only states for swim, carpet, slider, flight (`takeover.rs:1566-1580`). | Obsolete (confirm in game: KH2 air moves over Hook's sea and the Bottomless Hall) | Main's fix was for [CTRL]. | — |
| 31 | KH2 Cure's heal rule | `cure.rs`, `win/formmagic.rs`; s339 | `ceil(power × Magic / 100)` (rows 708 / 709, partners half), Magic = max MP × 3 × `magic_scale`. | s339 "using cure with KH2 magic selected barely heals sora" | `formmagic::heal` (`formmagic.rs:1185-1211`, t867): KH2's formula, Magic = the caster's max MP × 1, through KH1's own add-HP call (Steam `0x1402A4920`, bytes checked). | Ported (native) | Main's × 3 × `magic_scale` dropped (testing's magic calculations win). **Noted for later (owner: "cure should follow kh2's rules but leave it for now and note it off"):** Cure's Magic is max MP × 1 (t867) while t917–t920 put the other Magic-based hits on KH1's Ripple Drive stat (max MP × 3); Cure is to follow KH2's rules. | Later, when the owner picks it up. |
| 32 | Bonus AP | `growth.rs`, ini `bonus_ap` | Max AP grows by each KH2 ability's AP. | (README "HOW ABILITIES WORK") | Removed t718: "remove extra ap for now". | Superseded | Testing's rule (t718) stands. | — |
| 33 | Swings meet walls as KH1's do | `combat.rs`, ini `walls = kh1 / clink / pass`; s309 | KH1's wall spark and sound, no bounce, the swing keeps hitting enemies. | s309 "keep the clinking mechanic for enemies that should have it ... let sora hit the walls without clinking", "this is how Base KH1 handles sora hitting a wall" | Pre-s309 `wall_clink` (on by default, `settings.rs:77-78`, `combat.rs:1816-1831`): clang, the swing's further hits dropped; no bounce since t901. `bodyhits.rs:38` keeps KH1's wall test off for our body objects. | Missing | Testing's behaviour matches neither main's `kh1` nor `clink`. Native version: let KH1's own wall test run for KH2 swings; depends on the hit path being KH1's builder, not [SWEEP]. | Design round, then one version. |
| 34 | KH2 camera follows KH1's invert options | `camera.rs`; s303 | Inverted X and Y respected. | s303 (a player's report) | Left / right honoured (`camera.rs:84`, `TURN_SIGN_RVA`); up / down option never read. | Partly | None. | One small version (decomp check of KH1's option first). |
| 35 | Combo lunges don't carry Sora through room exits or into crate pushes | `win/gates.rs`, `docs/kh1-exits-push.md` | A lunging combo next to a door or crate doesn't change rooms or start a push. | (pre-s260, quoted in `gates.rs`) "a combo pushing you forward shouldnt bring you into a new room ... shouldnt go into soras push animation" | Testing's `gates::pre_move` is never called (only `gates::released()`, `takeover.rs:947`): dead code. | Missing (Assumption: whether KH1's attack state already blocks it; check in game) | Main's mechanism wrapped [CTRL]'s move. Native fix, if needed: KH1's own mover / event queue. | Check in game, then decide. |
| 36 | KH2 effects on moves | `kh2fx.rs`, `fx12.rs`, `kh2-fx`; s136, s161, s162, s172, s332 | KH2 effects on Explosion, Vicinity Break, Magnet Burst, Aerial Finish, Guard Break, Finishing Leap, Aerial Recovery, and (s332) Counterguard, Flash Step, Retaliating Slash. | s332 (a player: "Counterguard doesn't have it VFX") | All through KH1's own effect system (`kh2dpx.rs`, `kh2ppp.rs`, `dpxcache.rs`). Default list (`kh2fx.rs` `DEFAULT_MOTIONS`): 166, 170, 197, 194, 165, 167, 40, 203; **171, 169, 172 not in it** (they play only with `kh2_effects = all`). | Partly | Main's renderer [RENDER] is not ported. | One small version: the three motions in the default list (rebuilds the DPX cache). |
| 37 | Square's KH2 order on keyboard | s303 | Guard at rest, else Dodge Roll, else Quick Run, keyboard too. | s303 (a player: "you CANNOT use Quick Run while you have Guard equipped") | KH2's order on KH1's own Square press (`combomods.rs:2533-2790`, t858). | Ported (native) (Assumption for keyboard; check) | — | — |
| 38 | KH2 camera through door exits | s313 | No FOV glitch Coliseum Gates → Lobby; hand-overs blend. | s313 "several rooms in the game have this camera glitch with the KH2 style camera enabled" | KH2's camera as KH1's own camera mode (`camera.rs:16-31`); door scripts no longer count as a scene. | Superseded (Assumption; check that door in game) | — | — |
| 39 | Combo rows measure targets by their body; area moves see every foe | s302, s332 | Hercules' second hit doesn't jump to the air combo; Vicinity Break / Magnet Burst see all foes. | s302, s332 (players) | t889: KH2's walk measures KH1's lock points of every enemy (`combomods.rs:387-470`); lock height = radius (Assumption). | Superseded | — | Check Hercules and Vicinity Break with 2+ foes in game. |
| 40 | Neverland glide hang | s332 | — | (a player) | Native glide on KH1's states (`win/glide.rs`). | Obsolete | [CTRL] only. | — |
| 41 | Footsteps (walk / run only) | s155, s181, s205, s229, s313 | KH1's steps. | s313 "just keep KH1 walk steps in" | KH1's own motions play KH1's steps. | Obsolete (Assumption: KH2 slot motions add no step sounds) | — | — |
| 42 | Ability grants: held-back KH1 grants, repeated level-up abilities | s302, s316 | — | s316 "if explosion comes from defeating dragon maleficent, why did i get it" | Vanilla progression, split ids, all KH2 abilities at once (t718). | Obsolete | — | — |
| 43 | s330's seven player bugs | s330 | — | (players) | Controller bugs gone with [CTRL]; Reflect's voice by Battle Quotes (t583, t750) and the Keyblade vanish flash (`kh2fx.rs` `weapon_vanish`) exist natively. | Obsolete | — | — |
| 44 | Neverland flight with KH2 air attacks | s118–s123 | — | s118 "Sora couldn't fly in Neverland" | t648: KH2's air combo on KH1's flight attack. | Ported (native) | — | — |
| 45 | Magic carpet | s102, s111 | — | — | KH1-only state 29 (`takeover.rs:1566-1580`). | Ported (native) | — | — |
| 46 | Four shortcuts | s97–s99 | L1 + cancel is the fourth. | "four shortcut slots like KH2" | `win/shortcuts.rs`, ini `shortcuts = 4`. | Ported (native) | — | — |
| 47 | Reflect magic | s170–s182 | Reflect, Reflera, Reflega in the Magic menu and shortcuts, 1 MP. | s172 "Reflect should cost 1 MP" | `win/reflect.rs`: command 0x7D, cast on KH1's cast state, barrier as a KH1 guard object. | Ported (native) | Level rule: row 9. | — |
| 48 | Attack tutorial counts a KH2 attack | s144, s145 | Dive to the Heart's attack lesson ends with KH2 combos. | — | `combat.rs:197-206` and `probe.rs:80-93` read `ATTACK_SHOWN`, nothing writes it. KH1's own Attack step now starts the row's motion request (Assumption: the script's check sees it). | Obsolete (confirm in game; dead code to clean) | — | — |
| 49 | Dream weapons held rigid; Dream Shield wrist | s226, s250, s329 | No stretching. | s329 "having the dream shield equipped leads to some pretty bad stretching issues" | `pose.rs:950-965` `dream_hold` present; `kh1_wrist` (`pose.rs:892-935`) is pre-s329. | Partly (Assumption: matters only if the drawn path still runs; check the Dream Shield in game) | — | Check first. |
| 50 | Gummi route skip | s114, s252, s281 | — | s281 "comment it out" | `win/gummi.rs`, ini `gummi`, QoL row (t704, the owner: "put gummi skip in there"). | Ported (native) | Testing's t704 overrides s281. | — |
| 51 | Version in the Config menu | s154 | — | — | `config.rs:802`. | Ported (native) | — | — |
| 52 | KH2 sound volume | s151 | `kh2_volume`. | — | Applied in the banks built for KH1's engine (`kh2sound.rs:283-286`, t565). | Ported (native) | Main's waveOut player is deprecated. | — |
| 53 | KH2 camera in summons, flight, Simba's charge | s88, s90, s119 | — | "KH2's camera should stay through Simba's charge" | `takeover.rs:1580-1605`, `camera.rs:345`. | Ported (native) | — | — |
| 54 | "New Ability" box | s63, s65, s79, s80 | — | — | `menu.rs` `queue_prompt` (t718: one box for three or more). | Ported (native) | — | — |
| 55 | KH2 abilities replacing KH1's, learned by level and bosses | README "HOW ABILITIES WORK" | Vortex is Flash Step, and so on; KH2-only abilities by level. | — | Split ids (t284), vanilla progression and every KH2 ability given (t718), KH2 list (t722), main saves migrated once (`growth.rs` `migrate`, marks `0x4B02..0x4B04`). | Superseded | — | README rewrite at the merge. |
| 56 | `abilities = remove` (uninstall) | README "UNINSTALL" | Takes the mod's abilities out of the save. | — | `settings.rs:13-16`, `menu.rs:647`. | Ported (native) (Assumption: also empties the t722 KH2 list; check) | — | — |
| 57 | Player Model | s257–s260 | — | (testing owner: only a test, not a concern) | `player_model` kept, off. | Superseded | — | — |
| 58 | Hair, hood and chain physics | s185–s201; ini `physics`, `kh2_hair` | — | (owner: baked into the animations) | No hair switches (t388, t628). | Superseded | `physics` key reused on testing (section "Ini"). | — |
| 59 | Time profile | `win/prof.rs` | Logged averages. | — | `win/loadprof.rs` (t783), debug only. | Superseded | Timed deprecated sections. | — |
| 60 | `magic_scale` | ini, README | Extra multiplier for Explosion / Magnet Burst. | — | Removed t917; KH2 Magic-based hits on KH1's magic stat (t917–t920). | Superseded | — | — |
| — | Reaction commands | main `docs/reaction-commands.md` only (code in `kh2-bevy`, the playground) | Nothing: main's KH1 mod never had them. | — | Testing has a native research plan (`docs/reaction-commands.md:93-190`, `testers/reaction-commands/`). | Not a main feature | — | — |
| 61 | Epic Games Store support | `win/addr.rs`, `win/steam_map.in`, `docs/steam-epic-addresses.md` | Main runs on Epic and Steam. | — | `addr.rs` kept; old `Rva` hooks run on Epic; the hundreds of newer Steam-only hooks are off there (`engineering-rules.md`: Epic "once the Steam work is done"). | Partly | Merge gate, owner's call. | Owner's call. |
| 62 | README / `openkh/README.txt` / `mod.yml` | — | What players are told (controls, settings, abilities, Epic). | — | Out of date for testing's features. | (merge gate) | — | At the merge. |

---

## Ini (section 5.1): deprecated by the owner

The owner: main's ini settings are deprecated. **No carry-over and no converter.** Main's keys that testing
doesn't have (`walls`, `music_mute`, `magic_scale`, `kh2_hair`, `fx_test`, `bonus_ap`, `glide`, `hud_toggle_key`,
the Drive gauge keys) are ignored; a ported feature gets testing's own Config row and key when it is built.
Testing keeps every unknown line when it saves (`settings.rs:904-924`, Checked (code)), so nothing crashes.

**Keys that did nothing on testing: removed in t924** (Checked (code)):

| Key | What testing does with it |
|---|---|
| `carry`, `ceilings`, `grabs`, `swing_fx` | Parsed (`settings.rs:718-722`), nothing reads the value |
| `air_cast_fall` | Parsed (`settings.rs:734`), nothing reads it |
| `air_magic` | Read (`settings.rs:821`), sets nothing |
| `glide` | Already not read (commented out since t319, `settings.rs:730-733`) |

t924 also checked every other key in testing's default ini: each has a reader (a key-by-key search, Checked (code)).

Two notes from the check, for testing's own rows: `physics` means kh1 / kh2 movement physics on testing
(main's hair-sway `on` / `off` both read as kh2, harmless). Testing's six style keys default to kh1 when the
line is missing. The Config rows main had all exist on testing's sub-lists, except Aerial Magic (no row).

## Save (section 5.2)

The owner: a main save is reconfigured on load so every ability matches testing's layout. Testing
already migrates older layouts once (`growth.rs` `migrate`, `move_to_kh2_list`, marks `0x4B02..0x4B04` at
`+0x48A`); main's support abilities (78..81) and main's `+0x48B` use are what that migration doesn't know yet.
The conversion is its own version near the end, after its research round.

Offsets in KH1's save block (`menu::save_block`). Checked (code) unless marked.

| Offset | Main writes | Testing uses | Clash? |
|---|---|---|---|
| `+0x37` | Drive bars (low 7 bits, `0x80` = written; unset reads as full), s318 | Not used | Free |
| `+0xAB` (`0x74 + 0x37`) | Drive points 0..100 | Not used | Free |
| `+0x44..` (Sora's 48 ability bytes) | **Support ability ids 78..81** (Damage Drive, Form Boost, Damage Control, Drive Converter; bit 7 = off) | Ids 78..81 are **Flash Step, Aerial Sweep, Counterguard, Finishing Leap** (`growth.rs:356-359`); `move_to_kh2_list` moves them into the KH2 list (`growth.rs:890-913`) | **Clash.** A main save with support abilities reads as the wrong abilities on testing (Assumption from code: they'd be moved into the KH2 list as Flash Step and so on, and `migrate` would skip giving Flash Step for Vortex because id 78 is "already there", `growth.rs:857`). **Owner: reconfigured to testing's layout** (new ids for the four in the KH2 list). |
| `+0x488` | Valor's Keyblade (shared since s212); main can store the Dream Sword `0x52` (s306) | `equip::VALOR_OFF`, same use | Same byte; a stored 0x52 is refused on testing and falls back to the strongest Keyblade (no loss) |
| `+0x489` | Final's Keyblade (s283) | Not used | Free |
| `+0x48A..+0x48B` | `+0x48B`: Master's Keyblade (s300) | **Testing's ability-layout mark, a u16 at `+0x48A`** (`growth.rs:803-813`, `0x4B02` / `0x4B03` / `0x4B04`) | **Clash.** Main's own note (main `equip.rs:77-80`) says the mark must shrink to a u8 before a merge. A main save with a Master Keyblade breaks the mark's high byte, so testing's layout migrations may run again on it (Assumption). The conversion must read a main save's Master Keyblade before testing's mark is trusted, and Master's slot moves to a free byte in testing's layout. |
| `+0x48C` | Read only (Dive choice) | Testing names `+0x48C / +0x48D` growth bytes (`equip.rs:49`) | No write clash; check what testing writes there in the research round |
| `+0x499 + 0x91 / 0x92` | Drive Recovery / High Drive Recovery counts (KH1's inventory) | Not referenced | Free; keep the ids. On testing KH1 shows its own unused records for those ids (Assumption) |
| `+0x882 / +0x884 / +0x886` | Not written by main | Testing's form levels and EXP | No clash (a main save reads 0 → Lv1, Assumption) |

Read-only bytes main's story rules use: `+0x1504 + world`, `+0x16E3`, `+0xF36 + cup`, `+0xD41`, inventory.
Main's other mod data lived in memory only (item records 0x91 / 0x92, 17 chest entries, the prize row copy).

## Known conflicts

Settled by the owner (2026-10-29): forms' cost (a toggle, MP or the Drive gauge); the form gauge on testing's
form levels; abilities and saves (testing's layout, main saves reconfigured); the ini (deprecated, no
converter); bonus AP, music mute, Reflect levels, `magic_scale` (testing's systems stand); Cure (KH2's rules,
later).

Also settled: the owner's non-KH2 rules go behind a balance toggle and a progression toggle (on / off); the
secret code gives Anti.

Still open:

1. Epic support and the version letter after the merge.

## Not checked this round

- No Steam decomp or exe reads (every row's decomp check is Not started).
- Nothing checked in game.
- Testing's own systems that a port would build on and that may still be on a deprecated path were not audited
  here. Before Limit, Master and Final's magic ("Wisdom's way"), check how testing's KH2 spells run today
  (`formmagic.rs` and `reflect.rs` drive `kh2_magic`'s scripts through `kh2script.rs`, which says it runs on
  KH1's own script machine) against rule 2a.

## Research rounds

### Round 1: story flags (row 8: when each form opens, and the "New Drive Form" box), 2026-10-29

Full notes, addresses and the KH1 side: `docs/kh1-drive-forms-native.md` §12. Base t924.

**Testing now.** Valor and Wisdom are always in the Drive list, greyed only by `form::usable` (MP > 3, Valor
needs a spare Keyblade). Nothing in the save says which forms the player has. The form level cap counts
Valor and Wisdom as always owned (`formexp.rs:155`). Entering a form refills HP only. Checked (code).

**Main and the owner.** s279 / s307: forms open by the owner's table (Guard Armor, Opposite Armor, the
Pegasus Cup; order by the Dive choice; Master after Anti-Sora, Final after Riku 2), "dont use the
archipelago work just make it yourself". s308: "the game should tell you when you get a drive form". Now
behind the progression toggle (the owner, this port). Main re-read the story bytes on every Drive list
rebuild and guessed "same save" from byte jumps for the box.

**What KH2 does** (Checked (decomp), Steam KH2):
- Owning a form = owning its form item. `0x1403C3EF0(form)` gives the item from a table at `0x1405C5D08`;
  `0x1403C4720` tests it. Final's item 0x1D is visible in code; the others are data (Assumption).
- The Drive submenu `0x1403F3110` (and inline in the command menu builder `0x1403F29E0`) lists forms 1..6
  in order Valor, Wisdom, Limit, Master, Final, Anti, **only those owned**; nothing is shown for a form not
  owned. The partner rule `0x1404005E0` hides a form whose partner is missing and greys it when the
  partner is down. With no form listed (or a lock), the slot holds command 0x68 (what it shows: Assumption).
- Gaining a form item (`0x1403C3FE0`): Anti's counter reset to 0; for Valor, Master and Final an empty
  Keyblade slot gets the first owned Keyblade. No popup here; a form given as a bonus shows as a line in
  KH2's bonus window (`0x1403EDCA0` → `0x1403EDB20`).
- The form level cap is 2 + forms owned among Valor, Wisdom, Limit, Master, Final (`0x1403E8380`,
  `0x1403E88B0`).
- Owning forms does **not** raise the Drive gauge's maximum: only bonus levels do (`0x1403EDF90`, cap 9).
  Main's doc said otherwise.
- Entering a form (`0x1403C17C0`) refills HP **and MP**.

**How KH1 does it** (Checked (decomp), Steam KH1): KH1 has no forms. Story bytes at save `+0x1504`
(`0x1401B6B00`), journal at `+0x16E3` (`0x1401B5980`, `0x1401B6640`), Dive choice at `+0x48C` (written
by event command `0x1401B0380`, read only by the level-up `0x1402B51B0`; which value is Sword / Shield /
Rod: Assumption). Cup states at `+0xF36..`: not found in the export (Assumption, main's). KH1's level-up
box record at `0x14283B390` is what testing's `queue_prompt` already writes. Testing writes nothing at
`+0x48C / +0x48D` (Checked (code)), so reading the choice is safe.

**Deprecated in main's version.** Re-reading story bytes every rebuild (our check each frame) and the
"same save" guess. Replaced by a stored byte KH1's save carries: one read for the list, a bit set during
play means a gain.

**What KH1 lacks, the ability to add.** A form-owned field in the save: save `+0x881` (no Steam
instruction touches `+0x881..+0x887`, Checked (exe) in §9; unused by testing, Checked (code)). Bits 0..5
Valor..Anti (KH2's form ids − 1), bit 7 "set up". The box needs nothing new.

**1:1 checked.** List order and owned-only listing (`0x1403F3110`); level cap 2 + owned (`0x1403E8380`);
HP + MP refill on entry (`0x1403C17C0`); max bars not raised by forms (`0x1403EDF90`); story bytes `+0x1504`
(KH1 `0x1401B6B00`), journal `+0x16E3` (KH1 `0x1401B5980`). Main's fight thresholds and journal ids:
Assumption (from KH1's scripts, not rechecked).

**Missing from main.** Stored ownership; the partner rule in the list (hide vs grey; row 10); the level
cap from forms owned; the Keyblade put in at gain; Anti's counter reset; Final by the Anti roll after story
flag 0x4842 (`0x140421D60`); the pause menu's "new" marks (`0x140369520`); the 0x68 placeholder; KH2's
no-Drive locks and worlds (`0x1403A3A90`, `0x142A10504 / 08`); per-form drive counters (`0x1403E69A0`).

**Missing on testing** (also told to the owner in chat): the owned state and the list built from it; the
partner rule (row 10); **the MP refill on entry**; the level cap reading forms owned; Valor's slot filled
at gain and Valor allowed without a spare Keyblade (testing refuses); KH2's no-Drive locks.

**Plan, one version each.**
1. The owned byte at `+0x881`; a save without the mark gets the forms testing has (Valor, Wisdom; Final if
   its record is set), silently. The level cap reads it.
2. The Config menu's new Drive list with the progression toggle (and later the cost toggle). On: the owner's
   table sets owned bits in the once-a-second check `Growth::update` already runs (no new per-frame work).
3. The Drive list built from ownership in KH2's order.
4. The "New Drive Form" box when a bit is set during play (`queue_prompt`, main's text).
5. The form's Keyblade at gain (after an answer).
6. The MP refill on entry (its own version, after a go-ahead).

**Not found.** KH2's form item ids other than Final's (data); what command 0x68, `+0xDE0` values 7 / 0xC,
the world list `0x1405BAC00` and the world factor > 1.0 mean; KH1's cup bytes on Steam. Nothing in game.

**Questions.**
1. Progression toggle off: which forms does the player have? (a) every form testing has, from the start (as
   today); (b) a KH2-like order on KH1 fights (a new design); (c) none (no Drive). Recommend (a): today's play
   stays, and the list is KH2's owned-only either way.
2. Its default: (a) on; (b) off. Recommend (b): off keeps current saves and tests the same while the rules
   wait "to be worked on later".
3. Where the two toggles go: (a) progression in the new Drive list, balance in Combat > Tweaks; (b) both in a
   new list of their own (for example "Main Branch Rules"); (c) both in the Drive list. Recommend (b): they
   cover more than forms (gauge max, support levels, item rates) and stay easy to find.
4. Forms not built yet (Limit, Master; Final on hold): (a) store their bits, show nothing until the form
   exists, then the box once; (b) the box at the fight anyway; (c) leave them out of the table for now.
   Recommend (a).
5. Valor's second Keyblade (a KH2 conflict): KH2 fills the slot at gain and never refuses Valor. (a) fill
   `+0x488` at gain, keep testing's refusal when there's none; (b) fully KH2 (fill, and Valor allowed with
   one blade); (c) as now. Recommend (a): the gain part is KH2's, the refusal guards the two-blade moveset.
6. Saves already past those fights when the toggle goes on: (a) granted silently; (b) a box for each once.
   Recommend (a).
7. The Drive row with no form owned: (a) greyed (as today); (b) KH2's placeholder 0x68 after I check what it
   shows. Recommend (a) until 0x68 is checked.
8. The MP refill on entry: (a) its own version now; (b) later with the gauge. Recommend (a): KH2's, small.
9. KH2's drive counters and pause-menu "new" marks: (a) skip; (b) later. Recommend (a): KH1 has no form page
   to show them.

Write "go ahead" with the answers.

**The owner's answers (2026-10-29):** 1 (a) toggle off: every form testing has, as today. 2 (b) off by
default. 3: a new **Progression** submenu in the Config menu's main list for the progression toggle, and a
new **Drive** submenu there too. 4 (a). 5 (a). 6 (a). 7 (b) KH2's placeholder. 8 (a), as a toggle in the
Drive submenu. 9 (b) later. Then: the balance toggle (a), a row in the Progression submenu; "go ahead".

**Command 0x68 checked** (question 7): its 03system `cmd` row has kind 0 and message 0x8503, which in
`msg/us/sys.bar` is "?" (the same character ends "But what about Cerberus?", message 0x2A4). Checked
(files). KH2's state function `0x140401D60` gives a kind-0 command state 5 (greyed), Checked (decomp). So
with no form owned KH2's Drive slot is a greyed "?". The same rows give Valor's and Wisdom's cost 3 at
`+0x1C` (Checked (files)).

Plan with the answers (one version each): (1) the owned byte at `+0x881` and the level cap; (2) the
Config menu's Progression submenu (Story Unlocks, off by default; off = Valor and Wisdom owned as today) and
the owner's table on it; (3) the Drive submenu, its first row the MP refill on entry (on / off; on = KH2's
refill); (4) the Drive list built from ownership in KH2's order, with KH2's greyed "?" when none is owned;
(5) the "New Drive Form" box; (6) the form's Keyblade put in its slot at gain, Valor still refused without a
spare one. Drive counters and "new" marks: later (9 (b)).

**Built (go-ahead 2026-10-29, the balance row's place answered (a): a "Balance Changes" row in the
Progression submenu, added with the first balance rule that is ported).** t925 the owned byte and the level
cap; t926 Progression > Story Unlocks; t927 Drive > MP Refill; t928 the Drive list from the owned forms with
KH2's greyed "?"; t929 the "New Drive Form" box; t930 Valor's Keyblade slot at gain. Notes:
`docs/kh1-drive-forms-native.md` §12.8.
