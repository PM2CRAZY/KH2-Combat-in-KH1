aligned with main s339 / testing t948

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
| Ported (native) | 17 |
| Ported (running along) | 0 |
| Partly | 7 |
| Missing | 17 |
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
| 1 | Drive gauge data | `drivegauge.rs`, `win/gauge.rs`; s318, s323, s325, s326, s331 | Bars of 100 points. Max 3, +1 each for Sabor, Riku 1, Chernabog, Sephiroth, Kurt Zisa, the Unknown (cap 9). Costs Valor 3, Wisdom 3, Limit 3, Master 4, Final 5, Anti 9 (needs 3). Fills from hits (KH2 ATKP drive value; KH1 hits 3 physical / 5 spell), damage taken (Damage Drive only), orbs (1 / 3). In a form the gauge is the form gauge `(level + 3) × 600` frames, 1 a frame in battle, 0.5 outside. Full by default. MP cost and drain removed. | s318 "lets incorporate the drive gauge"; s323 "don't show the drive gauge until the first drive form is unlocked. By default, the drive gauge should be full."; s325 the six-fight table, "when in a form, have the drive capacity be raised to whatever level the form is"; s326 Chernabog instead of beating the game; s331 "Make limit form only cost 3 bars to enter" | No gauge; forms on MP. Planned as step 1 in `docs/kh1-drive-forms-native.md` §5 (KH1 hit contact `0x1402BF330`, HP change `0x1402A4920` noted on Steam). | Ported (native) (t931–t935; not checked in game) | **Settled (owner):** a Config toggle in a new Drive submenu picks MP or the Drive gauge as the cost. The form gauge uses testing's KH2 form levels 1..7 (KH2's `(level + 3) × 600`, 1:1); main's growth-level rule (s325) is dropped. Still open: the max table and Limit's 3 bars are the owner's, not KH2's. | Gauge data in the save + KH2's add rules + the hit add at KH1's contact; then the Drive submenu and its cost toggle (MP / Drive gauge); then the form gauge and drain. |
| 2 | Drive gauge HUD | `win/gaugehud.rs`, `win/kh2hud.rs`, kh2-formats `seqd.rs` `dds.rs` `png.rs`; s319–s324, s326; ini `drive_gauge_art`, `drive_gauge`, `drive_gauge_scale`, `hide_hp_label`, `drive_gauge_debug` | KH2's DRIVE bar, MAX plate, number, FORM bar in a form, left of / below Sora's portrait inside KH1's HP/MP task with KH1's sprite calls; optional KH2 `gaug` art from `zz0field.2dd`; only while the player controls Sora; KH1's "MP HP" label hidden. | s320 "use ghidra if you have to on KH2 to get the exact graphics, the animation while the gauge is charging, when its being drained, and all the animations"; s321 no rise-in animation; s322 "This should be the right size and positioning"; s324 bar outlines restored; s326 "only showing when the character is actively controlling sora" | Nothing. KH1's HUD task noted on Steam `0x14026DB00` (field meanings Assumption). | Missing | None. | KH1 pieces on KH1's HUD; then KH2's art built once into the mod folder (main decoded it every start [DECODE]); visibility from a KH1 signal (main used `takeover::script_has_sora` [CTRL]). |
| 3 | Drive orbs as KH1 prize types | `win/prize.rs`, `win/prizeart.rs`, kh2-formats `prize.rs`; s318, s328 | Prize types 7 / 8 in a copied KH1 prize row table; KH1's spawner, pickup, scatter and blink run them. Drop weights after KH1's own drop. KH2's orb sprite from `obj/PRIZE.mdlx` through KH1's texture lookup. | s318 "I want them to run through KH1's engine, not an overlay"; s328 "if possible, let's try to get the texture from KH2" | Nothing. Steam addresses already in `kh1-drive-forms-native.md` §3 (make `0x1402AB2A0`, drop `0x1402ABC40`, pickup `0x1402AC080`, rows `0x140528BE0`, Checked (decomp) there; they differ from main's). | Built t944–t948 (art: t947, t948) | Drop weights partly ours ("six small and two big"), not KH2's `przt`: check. | Prize types on KH1's prize code; then the art built once [DECODE on main]. |
| 4 | Drive Recovery / High Drive Recovery | `driveitems.rs`, `win/items.rs`; s326, s327, s333 | KH1's unused item ids `0x91` / `0x92` as KH2's items (+3 / +9 bars), pause-menu use only, bought 80 / 120; in item shops with a Potion (Traverse Town `tw10`, Agrabah `al19`) once a form is open; enemy drops (3% / 0.5%, MP orbs 5% / 0.75%, 20+ prizes 15% / 4%, × Lucky Strike); 17 chests patched (12 / 5). | s326 "have drive recovery items exist and be sold at the item shop in traverse town"; s333 "more available throughout the game (mob drops, chests, etc)" | Nothing (no 0x91 / 0x92 anywhere). Testing's `win/chest.rs` hooks KH1's chest Open (`chest.rs:211-225`). | Built t949–t952 | Rates, prices, chests and shops are the owner's design (KH2 never sells them). Chest table patching must coexist with `chest.rs` (Assumption: no clash). Ids must stay 0x91 / 0x92 for main saves. | Item records + shops; then drops and chests. |
| 5 | KH2 support abilities: Damage Drive, Form Boost, Damage Control, Drive Converter | `support.rs`, `growth.rs` `MENU` 78..81 + `SUPPORT_LEVELS`, `win/hurt.rs`, `win/gauge.rs`, `win/prize.rs`; s335, s337 | Learned at Sora's level 14 / 24 / 38 / 35, AP 3 / 5 / 5 / 5, KH2 names and help. Damage Drive damage / max HP × 50 a copy outside a form; Form Boost drain × 0.8 a copy; Damage Control halves damage at ≤ 25% HP; Drive Converter munny × 0.5, the rest orbs. Berserk Charge and Drive Boost left out (no MP Charge). | s335 the list, Berserk Charge and Drive Boost left out, "At set levels"; s337 "make drive converter come at level 35" | Nothing. **Ids 78..81 are Flash Step, Aerial Sweep, Counterguard, Finishing Leap on testing** (`growth.rs:356-359`, Checked (code)). Form Boost known only as an fmlv reward that "waits for the Drive gauge" (`growth.rs:583`). | Built t953–t957 (Drive Boost later) | **Settled (owner):** testing's ability rule; the four get new ids in testing's KH2 list, and a main save's 78..81 are reconfigured on load (section "Save"). Still open: testing's t718 vanilla-progression rule ("just give the player all the custom abilities") vs main's set levels. Drive Boost / Berserk Charge wait on the open KH2 MP / MP Charge thread. | After the gauge and orbs: one version for the four abilities (new ids). |
| 6 | Forced reverts (bars refilled, partners back), back to Sora in a new world | `win/form.rs` `revert(forced)`, `gauge::refill_bars`, `vanish::force_back`; s275, s308 timings, s338 | A revert outside the player's control (KO, new world, scenes, battle end, Coliseum arena) refills every bar and brings partners fully back; Revert command and gauge-out don't. | s275 "when leaving a world in a drive form ... he should be automatically reverted"; s338 "make sure the party members are fully visible, and fully refill sora's drive gauge" | Reverts only on KO / Revert / 0 MP (`form.rs:674-707`). No world, scene or battle-end revert (Assumption: none elsewhere). | Done by t934 / t959 (owner: KH2's rule, round 8) | None. Main's scene detection was [CTRL] (`takeover::scene_frame`, `script_has_sora`): KH1 signals needed. | One version, after rows 1 and 11. |
| 7 | Drive Forms across Olympus Coliseum rounds | `win/cup.rs`, `win/vanish.rs`; s315, s332, s334 | Arena entry reverts; a won round keeps the form (no drain in the victory scene); partners restored before KH1's victory wait; none left invisible after a cup. | s332 "if you enter the cup in a drive form you should be reverted before the first battle, but if you defeat a round in a drive form it should carry over into the next round"; s315, s334 bug reports | Nothing for forms. Testing's `ingameskip` rewires cup event function 512 in the same flow. | Built t961 | Must be checked together with `ingameskip`'s 512 hook. Main's op 103 fix existed only because [CTRL]'s vanish cleared 0x8000. | One version, after rows 6 and 11. |
| 8 | Form unlocks by the story; "New Drive Form" box | `story.rs` (pure), `win/menu.rs` `form_box`; s279, s307, s308 | First form after Guard Armor, second after Opposite Armor, third after the Pegasus Cup; order by the Dive choice (Sword: Valor, Limit, Wisdom; Shield: Limit, Wisdom, Valor; Rod: Wisdom, Valor, Limit); Master after Anti-Sora, Final after Riku 2. Read-only from the save (TT `+0x1504`, journal `+0x16E3`, cups `+0xF36..`, Dive choice `+0x48C`). | s307 the owner's table, "dont use the archipelago work just make it yourself"; s308 "the game should tell you when you get a drive form and reflect" | No gating: both forms usable whenever MP > 3 (`drive.rs:232-245`, `form.rs:316-323`). | Ported (native) (t925–t930; not checked in game) | Behind Progression > Story Unlocks (off by default). Limit and Master are stored and wait for their forms. | Done: t925 owned byte, t926 Story Unlocks, t928 Drive list, t929 box, t930 Valor's slot. |
| 9 | Reflect levels by the story; "New Magic: Reflect" box | `story.rs`, `win/reflect.rs::level`, `win/menu.rs` `reflect_box`; s307, s308 | Reflect / Reflera / Reflega by how many of Pot Centipede, Hercules Cup, Ice Titan are won. | s307 (as row 8) | Reflect follows KH1's Aero level (`win/reflect.rs:124-136`), main's pre-s307 rule. | Missing | Testing's rule (Aero level) stands under the owner's "testing's layout is the base"; main's boss rule only if the owner asks. | With row 8, only if asked. |
| 10 | Forms need their partners | `partners.rs`, `form::field_party`; s311 | Valor needs Goofy, Wisdom Donald, Master and Final both, Limit none; checked on entry (field party id `+0x4C`, KO bits). | s311 "make drive forms require their specific party members to cast" | Drive > Party Required (t958, `driveparty.rs`: KH2's availability rule `0x140400740`). | Built t958 | None. | — |
| 11 | Partners vanish during the form (and their HUD entry) | `win/vanish.rs`; s312, s315, s332, s334, s338 | The form's partner disappears (actor flags, HUD slot fade `0x140266CC0` Epic) and comes back on revert, events, room change, game over. | s312 "the party member associated with the drive from also disappears (along with their hud icon)" | Nothing. | Built t958–t959 | Main called `takeover::script_has_sora` [CTRL] four times; [EPIC]. | One version. |
| 12 | Limit Form | `win/limits.rs`, `kh2-magic` limit tests, `win/form.rs`, `win/tint.rs`; s287–s299, s314, s331, s336 | KH2's Limit moves (Zantetsuken, Ripple Drive, Hurricane Period, guard, Zantetsu Counter, Dodge Roll); black jumpsuit and jacket, yellow belt, black shoes; Magic becomes a Limit menu of KH1's five limits (run by KH1, with or without the abilities), KH2's own limits built and switched off (s297); KH2 FM's shortcut table; 3 bars; limits heal per hit with KH2's `hp_drain` and work out of battle; no revert mid-limit. | s291 "KH1 limits that are equipped shouldn't even prompt in any drive form"; s293, s296 the outfit; s297 "comment the KH2 limits, and for the moment put in the equivalent KH1 limits"; s299 Limit menu, not Triangle prompts, KH2's default shortcuts; s331 3 bars; s336 "he should get health back ... use the limits whenever, even when out of combat" | None (`form.rs:79-83`; `docs/kh1-kh2-limits.md` §1). The `testers/drive-forms/` thread still says Limit runs on the old controller: stale. | Missing | 3 bars and KH1 limits instead of KH2's are owner overrides of KH2 (Config toggle or keep: ask). Main ran on [CTRL], [SWEEP], [SCRIPT]. **Rule 2a: testing has no native Limit Form to build on; the form's moveset has to be baked onto KH1's states first, the way Valor and Wisdom were.** | Moveset on KH1's states; look; Limit menu of KH1's limits; heal per hit and out of battle. |
| 13 | Master Form | `win/form.rs`, `win/equip.rs` M: row; s300, s301 | KH2's Master moves and combos, Aerial Dodge one above Sora's, Wisdom's magic rules, floating second Keyblade, M: equipment row (save `+0x48B`), yellow look; its KH1 Gravity travels with the cast's root. | s300 "take the rules and procedures we've done for wisdom form and valor form, and lets make master form"; s301 KH1 magic "doesn't actually move sora at all" | None. | Missing | Save byte `+0x48B` clash (section "Save"). Main on [CTRL] / [RENDER]. | After Limit, on KH1's states. |
| 14 | Anti Form (and its look) | `win/form.rs`, `win/tint.rs`, `kh1-data/src/texmap.rs`; s272–s292, s285 | By the secret code; free (later 9 bars, needs 3), no revert, 1.5× damage taken, no guard, no growth; magic, items, summons, limits, Drive refused; Anti Glide and Side Spin; ledge hang by the foot; no Keyblade or trails; KH1 Anti-Sora's textures on Sora's HD textures, solid yellow eyes RGB (253, 230, 95). | s272 "I want 1:1 combat parity. I dont want it to cost any MP to enter, and I dont want sora to be able to revert out of it"; s273, s277, s280 the ledge hang; s283 no guard; s285 "Antiform should take half of your MP"; s290 "copying the exact textures of KH1 anti-sora onto sora's anti form model"; s292 the eyes | None. Testing's secret code still gives Final (`form.rs:204-227`); no `texmap.rs`; `tint::Look` is Wisdom and Valor. | Missing | **Settled (owner):** the secret code gives Anti. Anti free / no revert goes behind the balance toggle (KH2's Anti counter and cost with it off; the counter wasn't used by main). Texture map built every start (~3 s) [DECODE]. | After Master, on KH1's states. |
| 15 | Final Form | `win/form.rs`, `kh2-rig/tests/final_form.rs`, `kh2-anim/examples/final_blades.rs`; s231–s283 | Secret code (s231), then in the Drive list for 3 MP with the code doing nothing (s271); white outfit, black belt; Wisdom-style magic; combo effects on the floating blades; F: equipment row (save `+0x489`). | s271 "put it into the drive menu, and have the cheat code do nothing for the moment"; s283 "add a section in the equipment menu for the second keyblade in final form" | `Kind::Final` exists with the code but is refused on Steam (`form.rs:125,363`, t873); second blade fixed at the Kingdom Key (`valorstats.rs:64`); no Final tint. Owner on testing: "we're not going to touch final form unless I change my mind" (`kh1-drive-forms-native.md` §10), "we're going to transfer it over when ready" (t873). | Partly | On hold by the owner. Main s271 / s272 vs testing's code. | Owner's call. |
| 16 | Forms' growth abilities | s270, s301 | One level above base Sora's, even if unequipped. | s270 "always be one level higher than the level unlocked by base sora" | KH2's fmlv: growth by form level (Lv1/3/5/7 → LV1/2/3/MAX), `form.rs:446-457`, the tester's answer 6 (b). | Superseded | Testing's KH2 1:1 rule. | — |
| 17 | Drive heals | s230 (before the merge base) | HP full on entry. | — | `heal_full` (`form.rs:275-287`) in `native_start`. | Ported (native) | KH2 also refills MP on entry (the drive-forms thread, m1): testing refills HP only. Check in the decomp. | — |
| 18 | Talking / examining in forms | s263 | Sora can talk and open / examine in a form. | s263 "let sora be allowed to talk to people and interact with objects in drive forms" | Valor lets Talk, Examine, chests, save points and object commands through (`formcmds::valor_allows`, t962). | Built t962 | None. | — |
| 19 | No limits in any form | s278, s291 | No KH1 limit offered or usable in any form. | s278 "sora should not be able to use limits in ANY drive form including anti form" | Drive > Limits in Forms (KH2 default: KH1's offer off for the form, `formlimits`, t963). | Built t963 | Owner's toggle (answer 2 (b)); Valor refuses them on either value. | — |
| 20 | Second-Keyblade equipment rows and rules | `win/equip.rs`, `win/equipmenu.rs`; s212, s220, s262, s283, s286, s306 | V: (Valor) row; F: and M: rows; the compare box shows the form's stats (s262); a slotted Keyblade counts as in use, also for the shop's sell list (s286); the Dream Sword is Valor's default (s306). | s262 "it doesnt accurately display its stats"; s286 "you able to have the same secondary keyblade for multiple forms"; s306 "i want the Dream Sword to be in the second keyblade slot by default" | V: row; compare box (t968), in use through KH1's free count (t967), empty slot = Sora's own (t966), `win/valorslot.rs`. | Built t966–t968 (F: / M: with their forms) | Dream Sword default vs testing's Keyblade-only rule. | Compare box; in-use count; default (after an answer); F: / M: with their forms. |
| 21 | Grey screen tint during a Drive change | s332 | KH2's grey tint during the change. | (found by main as missing) | KH1's light list and fog fade (t964) and KH1's own feedback blur (t965), `win/drivedim.rs`. | Built t964–t965 | None. | — |
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


### Round 2: the Drive gauge's data and rules (row 1), 2026-10-29

Full notes: `docs/kh1-drive-forms-native.md` §13. Base t930.

**Corrections first.** §2's pref numbers (600 a bar, 0.5 out of battle) are now Checked (files). §6 said
KH2's Anti counter and roll weren't found: they are in `0x140421D60`. §12 called command kind 0x48
"Limit-type": it is the summons (3 bars, paid the same way). Round 1's command 0x59 is the summon that takes
a form's look (Assumption which).

**Testing now.** No gauge. Forms cost 3 MP, drain one yellow MP every 3 battle frames and revert at 0 MP
(`win/form.rs`). `character_core::drive` (KH2's rules, Epic-derived) is on testing but unused. Checked (code).

**Main and the owner.** s318 "lets incorporate the drive gauge": KH2's costs in bars; filled by hits, damage
taken, orbs; KH2's form gauge. s323 "By default, the drive gauge should be full" and hidden until the first
form. s325 / s326 the maximum: 3, +1 each for Sabor, Riku 1, Chernabog, Sephiroth, Kurt Zisa, the Unknown
(cap 9). s331 Limit 3 bars. s338 forced reverts refill the bars. Testing's decisions: the cost by a toggle in
Drive; the form gauge on testing's form levels; progression and balance rules behind toggles.

**What KH2 does** (Checked (decomp), Steam KH2):
- A new game starts full, 3 of 3 bars (`0x1403A0E70`). The gauge lives in the player's status record and
  is saved at save `+0x1038..+0x1043`; a form and its time left survive room changes and saves.
- Hits: the ATKP's drive byte (`+0x29`), on enemies only, × 0.75 on normal enemies, × 2 (× Drive Boost) while
  MP charges, × a world factor, truncated, through the add `0x1403A9CF0` → `0x1403C0AD0` (100 points a bar,
  held at the maximum). In a form a hit takes 15 × the value off the form gauge (× 0.2 on normal enemies).
- Damage taken gives nothing without Damage Drive (its factor starts at 0, +50 a copy; row 5).
- Paying (`0x1403C2780`): bars minus the cost, floored at 0; the points are cleared when the gauge was full
  or short. Costs (03system `cmd`, Checked (files)): Valor 3, Wisdom 3, Limit 4, Master 4, Final 5, Anti 9,
  summons 3.
- The maximum rises only by bonus levels (`0x1403EDF90`, cap 9); a full gauge stays full as it rises.
- Entering: HP and MP full, the form gauge (level + 3) × 600 ticks (Anti as level 3). The drain is 1 a tick
  in battle, 0.5 outside, × 0.8 per Form Boost, none while an event holds (Assumption which flag). Empty:
  the Revert command (`0x140404A40`); the bars aren't touched.
- The only full refill is the recovery request `0x1403A5650`: going to the world map in a form (refill and
  revert), scripted map jumps, area programs, a Continue-type menu (which triggers: Assumption). No save
  point or level-up refill in code.

**How KH1 does it** (Checked (decomp) / (code)): KH1 has no gauge. KH1's contact `0x1402BF330` is already
hooked (`probe::hit` → `formexp::hit`); the ATKP's drive byte is reachable there through `kh1rows`
(`kh2_atkp`, `native_atkp`) and `bodyhits` (`combat::landing` carries only power: the byte has to be added).
KH1's own MP-on-hit has a level factor KH2's add doesn't, so it isn't reused. The drain fits in
`form::native_frame`, which already runs each frame of a form and reads KH1's battle flag. Damage taken would
go at KH1's HP change `0x1402A4920` (with Damage Drive). Save: testing uses neither `+0x37` nor `+0xAB`;
KH1's save field list marks `+0x37` a char field and no Steam KH1 code in the export reads it (a full exe
scan, as for `+0x881`, still to do).

**1:1 checked.** Costs, the add, the pay, the maximum, the entry, the drain and its numbers (sstm `+0x68`
600, `+0x6C` 0.5, `+0x70` 15, `+0x78` 2.0 Checked (files)), the revert on empty, the start full. Not checked:
the world factor's default (taken as 1.0 for KH1), the normal / boss split's names.

**Missing from main.** The world factor; a full gauge staying full when the maximum rises; a form and its
time carried over room changes and saves; KH2's refill rules (main refilled on every forced revert, s338);
the Anti / Final roll details; no drain during events; KH2's no-gauge mode 4; Limit at 4 (main 3, the owner's).
Main's per-frame HP-difference check for damage taken was a deprecated per-frame hook.

**Missing on testing** (told to the owner in chat): the whole gauge (record, save, add, pay), the Drive list
greyed by bars, the form gauge and its drain, hits draining a form, the revert on empty, the refills, and no
MP cost or drain when the gauge pays.

**Deprecated in main's version.** The HP-difference check every frame (replaced by KH1's HP change, with
Damage Drive); the hit add read from our own hit records (replaced by KH1's contact with the ATKP byte);
`takeover`'s scene signals (replaced by KH1's own, with the refills).

**What KH1 lacks, the ability to add.** A Drive gauge: bars, points and maximum in Sora's save, KH2's add at
KH1's contact, the pay, the form gauge in the form's own frame. Written up in §13 for the HUD (row 2), orbs
(row 3), items (row 4) and the support abilities (row 5).

**Plan, one version each.**
1. Gauge data and save (bars `+0x37` with bit 7 "written", unwritten reads full; points `+0xAB`; the maximum
   derived). No change in play.
2. Drive > Form Cost (MP | Drive Gauge): with the gauge, KH2's costs, the list greyed by bars, no MP cost or
   drain.
3. The hit add at KH1's contact (KH2's rule).
4. The form gauge, its drain, hits draining it, the revert on empty.
5. The refills.
6. Progression > the owner's six-fight maximum.
7. With Limit Form: Balance Changes > Limit 3 bars.

**Not found.** A KH2 save-point refill (save points are likely object-script data); what triggers each
recovery kind; the world factor's default; form id 0xC; two values the decomp drops; on KH1, a boss signal,
the world-exit and Continue signals, and KH1's tick rate in the decomp itself. The Anti chance rows are in
00battle.bin (needed for the Anti round).

**Questions.**
1. Form Cost's default: (a) MP (as today); (b) Drive Gauge. Recommend (a) until the gauge shows on the HUD
   (row 2).
2. The maximum with the six-fight row off: KH2's own rule would leave it at 3 for good (KH1 has no bonus
   levels), so Final's 5 could never be paid. (a) 3; (b) 9; (c) another rule. Recommend (b), as Story
   Unlocks off gives every form; the doc says it isn't KH2's number.
3. The six-fight maximum: (a) its own Progression row; (b) part of Story Unlocks. Recommend (a).
4. Refills: (a) KH2's (leaving a world in a form, a Continue); (b) also main's s338 forced-revert refill;
   (c) KH1's save points too. Recommend (a) now, (b) asked again with row 6.
5. Before any form is owned: (a) the gauge runs as KH2's; (b) it stays off. Recommend (a) (s323's hiding is
   the HUD's, row 2).
6. KH1's own hits (no ATKP): (a) KH1's attack data `+0x2C` (swings 3, finishers 6, spells 0, its MP-charge
   value); (b) main's 3 / 5; (c) nothing. Recommend (a).
7. KH2's normal-enemy factor: (a) find KH1's boss signal and use it, (b) every KH1 enemy normal, (c) every
   one a boss. Recommend (a), with (b) until the signal is found.
8. MP when the gauge pays: (a) no MP cost and no MP drain (KH2); (b) keep the drain. Recommend (a).
9. Save bytes: (a) main's `+0x37` / `+0xAB` after the exe scan (main saves keep their gauge); (b) new bytes.
   Recommend (a).
10. A form across a room change: (a) carried over (KH2); (b) reverted. Recommend (b) for now (testing's
    forms rebuild KH1's tables per actor; carrying over is its own work).
11. The Anti and Final rolls: (a) later, with Anti; (b) now. Recommend (a).

Write "go ahead" with the answers.

**The owner's answers (2026-10-29):** 1 (b) Form Cost's default the Drive gauge. 2 (b) maximum 9 with the
six-fight row off, **and bonus levels come later** (the owner: "we'll add bonus levels later, take a note of
it"; KH2's only way to raise the maximum, `0x1403EDF90`; to do: a KH1-native bonus-level system). 3 (a) its
own Progression row. 4 (a), and a three-value row in Drive, Off | KH2 | Save (the owner: "there's also an
exploit where if you leave a world while being in a drive form your drive gauge gets max refill, add it as a
3 toggle in drive"). 5 (a). 6 (a). 7 (a). 8: asked to clarify (below). 9 (a). 10 (a), as testing's Valor and
Wisdom already work (the owner: "check and see how they implement their drive forms and do the same for the
other forms"). 11 (a).

**Question 10 checked (code):** testing's form state is the Drive record in `win/form.rs` (`DRIVE`, `CURRENT`,
`IN_FORM`), not tied to the actor; each new actor's motion requests read the form at each lookup
(`kh2slots`, `form::native_form`, `redirect`), so a form already carries over room changes. The form gauge's
time goes in the same record and carries over with it; every later form (Limit, Master, Anti, Final) uses
the same path (`form::Kind::native_capable`, §7.5). Not saved in the save file (KH2 saves it): a loaded save
starts out of a form (Assumption, not checked in game).

**Question 8, restated.** Today a form costs 3 MP, then drains one yellow MP every 3 battle frames and reverts
at 0 MP. With Form Cost on the Drive gauge: (a) KH2's way: entering pays bars only, MP is untouched while in
the form, and the form ends when its form gauge runs out; (b) bars to enter, and the MP drain and the revert
at 0 MP stay as today (two timers). Recommend (a).

**Question 4's row, to confirm.** Drive > Gauge Refill: Off (no refill), KH2 (KH2's: leaving a world in a form
refills every bar and reverts), Save (Assumption: also refilled at a KH1 save point). Default KH2.

**Answers 8 and 4 (2026-10-29):** 8 (a) KH2's way; 4: "just have it be off or kh2". "keep checking": every value
rechecked in the Steam KH2 decomp as it was built.

**Built (go-ahead 2026-10-29):** t931 the gauge's data in Sora's save (`+0x37` / `+0xAB`, main's bytes;
an objdump scan of the Steam exe found no access by the save's pointer); t932 the hit add at KH1's contact
(KH2's ATKP drive byte, KH1's own `+0x2C` for KH1's moves, × 0.75 normal); t933 Drive > Form Cost (Drive
Gauge by default): KH2's costs in bars, the form gauge, hits draining it, the revert on empty, no MP; t934
Drive > Gauge Refill (Off | KH2): KH2's refill and revert on a world left in a form; t935 Progression > Drive
Gauge Growth (the six fights; off: 9). Notes: `docs/kh1-drive-forms-native.md` §13.10. Still open: bonus
levels (the owner, later); KH1's boss signal for the normal-enemy factor; the gauge on the HUD (row 2);
KH2's other refill kinds (Continue, map jumps); Form Boost and Damage Drive (row 5); the Anti / Final rolls.

### Round 3: the Drive gauge on KH1's HUD (row 2), 2026-10-29

Full notes: `docs/kh1-drive-forms-native.md` §14. Base t935.

**Testing now.** The gauge's data and rules (t931–t935) and nothing that draws it; the log shows it.

**Main and the owner.** s319–s324, s326: KH2's own art left of / below Sora's portrait inside KH1's HP/MP
gauge task; s321 "I dont need the animation of the drive gauge rising up"; s322 "This should be the right
size and positioning of the gauge" (KH2's unit × 1.34, 6 HUD units right, 1.3 lower); s320 / s322 the "MP HP"
label removed; s323 hidden until the first form; s324 the bars' outlines kept; s326 "only showing when the
character is actively controlling sora". Main decoded the art at every start (deprecated) and used the deleted
controller's signal for visibility.

**What KH2 does** (Checked (decomp), Steam KH2; the groups Checked (files)): the gauge is part of the player's
gauge object (`0x140ABD248`, made by `0x14016C3A0`), its pieces off the HP ring (`+0x1D40`); setup
`0x14017F9B0`, the update `0x140181270`, the per-mode logic `0x1401816C0`, the form change `0x140180900`.
Normal: the bar 21 slides in once, then 23 (26 while MP charges), the fill 25, MAX 20; in a form 50 / 51,
fill 49; summons 140 / 141 / 139; locked 18 / 19; digit groups 28–37, 54–63, 144–153. The fill isn't scaled:
a mask turns over it (about 23° a bar, twice that full). A bar gained plays 22 and system sound 0x20, MAX 20
and sound 0x21; a form change the sparkle ring 130 on the portrait, the revert 53 on the bar. No form-owned
rule: it shows whenever the player's gauge object is built, and hides with the whole HUD in events and menus
(`0x140168B50` / `0x140168B70`). The `gaug` sheet is image 2 of 4 in `zz0field.2dd`; its HD image is
`US_zz0field_2dd1.png`, 1024×512 (2× the PS2 size; the others are 4×).

**How KH1 does it** (Checked (decomp) / (exe)): Sora's HP/MP task `0x14026DB00` loops over 5 slots (records
`0x142811A00 + n × 0xDF0`, slot 0 Sora); with the event bit (`0x142867364 & 8`) it skips the gauges and the
party's alpha drops; per slot the body `0x14026CF90`, then the portrait `0x14027F470`; `0x14026D990` is the
slot's place. KH1's sprite draw is `0x14026CE90 (sprite, texture, x, y, packet, alpha, keep)`; textures are
TIM2, found and mapped through `0x140274060` / `0x14010BDF0` / `0x1400E2B20` / `0x14010DC50`, the same loaded-
file path `kh2dpx` uses for KH2's effect textures. The "MP HP" label comes from a layout file (its record:
not identified yet). First bytes of the 17 functions involved are in §14.

**1:1 checked.** KH2's pieces, groups, sounds and when they play; that KH2 has no form-owned rule. Not
checked: KH2's digit tables, mask factor and animation clock (data the C export lacks: the Steam KH2 exe would
show them).

**Missing from main.** The mask sweep; sounds 0x20 / 0x21; the revert flash 53; mode 0 (no gauge) and mode 4
(locked); MP Charge's bar 26; the summon pieces.

**Deprecated in main's version, and the replacement.** The art decoded at every start → built once into the mod
folder (`kh1-kh2-cache`, as `dpxcache` / `tintcache`); the controller's "has Sora" signal → KH1's own gauge
visibility (its event bit, slot and alpha); the per-frame label swap → a one-time data change.

**What KH1 lacks, the ability to add.** A Drive gauge piece on its HUD: one hook on `0x14026CF90` for slot 0,
drawing with KH1's own sprite call, slot 0's alpha and place; the art as a KH1 texture in KH1's loaded files.
Per frame: only while KH1 draws Sora's gauge, about 5 animation players and 6–20 quads (Assumption: not
measured).

**Plan, one version each.**
1. **t936** the art cache (pure, unit tested): BAR/SEQD/PNG reading in our own code, the cache file,
   built at the loading screen; log only.
2. **t937** the texture in KH1: the cached image entered in KH1's loaded files and mapped at its own GS
   address through KH1's `0x140274060` / `0x14010DC50`; log only.
3. **t938** the draw: the `0x14026CF90` hook, KH2's steady pieces (bar 23 / FORM 51, fill 25 / 49 with
   KH2's mask sweep, number, MAX 20), Drive > the HUD row.
4. **t939** KH2's animations: bar gained 22 + sound 0x20, MAX + 0x21, form change 130, revert 53.
5. **t940** KH1's MP HP label (after the answer).
6. **t941** the KH1-pieces fallback (only if chosen).

## 5. Questions (only what changes what gets built)

1. **Art**: (a) KH2's art only, built once; without it no gauge and a log line; (b) KH2's art with a
   KH1-pieces fallback; (c) a Config choice KH2 art | KH1 pieces. Recommend (a): KH2's files are
   already needed for KH2 Sora, so a missing gauge sheet means a broken install, and a second look is
   a second system to keep 1:1.
2. **Place and size**: (a) KH2's own layout: the bar at KH2's offset from its portrait (−79, +24 KH2
   units) scaled by KH1's portrait to KH2's, the number on the ring; (b) main's numbers: s322 "KH2_UNIT
   × 1.34, the gauge 6 HUD units right and 1.3 lower" on s321's "KH2 units are 1.2× larger ... the
   bar's middle sits 0.3 of the portrait's radius below the MP HP label's middle" (the owner, s322:
   "This should be the right size and positioning of the gauge"); (c) (a) with Config nudge rows.
   Recommend (b): it is the owner's measured choice, and KH2's own offset is computed from a portrait
   of a different size; (a) is the 1:1 alternative if you'd rather.
3. **The slide-in** (KH2's bar rises in when the HUD is made, groups 21 / 50): (a) left out (the owner,
   s321: "I dont need the animation of the drive gauge rising up"); (b) KH2's; (c) a row. Recommend (a).
4. **KH1's MP HP label**: (a) hidden while the gauge shows (s320 / s322 "remove it"), done once as data
   (the label's records given a size of 0 or KH1's plain plate, so the MP bar's bottom edge stays, the
   s324 problem) instead of main's per-frame swap; (b) kept, the gauge placed under it; (c) a row.
   Recommend (a). First the label's record has to be identified in game (a one-time log of Sora's
   frame sprites), since the layout comes from a file.
5. **When it shows**: KH2 has no form-owned rule (1.5). (a) whenever KH1's gauge shows (KH2's rule);
   (b) only once a form is owned (s323 "don't show the drive gauge until the first drive form is
   unlocked", read from the owned byte `+0x881`); (c) (b) only with Progression > Story Unlocks on.
   Recommend (b): with Story Unlocks off every form is owned from the start, so it plays as (a); on, it
   keeps s323.
6. **Beyond KH1's own HUD rules** (s326: "only showing when the character is actively controlling
   sora"): KH1 already hides its gauge in cutscenes and off the field. (a) follow KH1's gauge only,
   then check in game what's left (talks, scripted walks, the pause menu); (b) also hide while KH1's
   event-running word `0x14298C9A0` is set (one read a frame); (c) main's whole list. Recommend (a):
   native, no per-frame guess; anything left gets its own small version with a KH1 signal.
7. **Config rows** (testing's Drive submenu): (a) one row, Gauge HUD On | Off (On by default; the label
   follows it); (b) also a label row; (c) none (always on). Recommend (a). Main's ini keys
   `drive_gauge_art`, `drive_gauge`, `drive_gauge_scale`, `hide_hp_label`, `drive_gauge_debug` are not
   carried over.
8. **KH2's sounds 0x20 / 0x21** (missing from main): (a) KH2's, from KH2's system bank through the
   sound cache (KH1 has no sound for these, the owner's rule for menus); (b) none. Recommend (a)
   (whether the sound cache holds KH2's system bank: Assumption, to check).

Write "go ahead" with the answers.

## 6. Owner decisions already given (quoted from main AGENTS.md)

- s318 "lets incorporate the drive gauge"; "For the drive orbs, I want them to run through KH1's
  engine, not an overlay".
- s319 "I dont see the drive gauge at all" (then a mockup: KH2's gauge under KH1's HP circle).
- s320 "the drive gauge should be here instead ... remove [the HP/MP labels] from the hud if possible
  ... use ghidra if you have to on KH2 to get the exact graphics, the animation while the gauge is
  charging, when its being drained, and all the animations that the hud element has".
- s321 "I dont need the animation of the drive gauge rising up. Also the drive gauge is off from where
  it should be. its also kinda small".
- s322 "it could be a bit bigger just slightly. Also, the bottom black line of the MP bar (right above
  the drive gauge) is cut off. also, the "HP" text is still there, remove it", then "This should be
  the right size and positioning of the gauge".
- s323 "don't show the drive gauge until the first drive form is unlocked. By default, the drive gauge
  should be full."
- s324 "remove the numbers in the drive menu ... the outline of the MP and HP bars look like they've
  been cut off".
- s326 "make sure the drive gauge is only showing when the character is actively controlling sora"
  (main's code note adds: "Dont show it on the world map, in the menu, in the gummi ship routes,
  during cutscenes, etc").
- Testing round 2, Q5 (a): the gauge runs before any form is owned (the HUD's hiding is this row's).

**Questions.**
1. **Art**: (a) KH2's art only, built once; without it no gauge and a log line; (b) KH2's art with a
   KH1-pieces fallback; (c) a Config choice KH2 art | KH1 pieces. Recommend (a): KH2's files are
   already needed for KH2 Sora, so a missing gauge sheet means a broken install, and a second look is
   a second system to keep 1:1.
2. **Place and size**: (a) KH2's own layout: the bar at KH2's offset from its portrait (−79, +24 KH2
   units) scaled by KH1's portrait to KH2's, the number on the ring; (b) main's numbers: s322 "KH2_UNIT
   × 1.34, the gauge 6 HUD units right and 1.3 lower" on s321's "KH2 units are 1.2× larger ... the
   bar's middle sits 0.3 of the portrait's radius below the MP HP label's middle" (the owner, s322:
   "This should be the right size and positioning of the gauge"); (c) (a) with Config nudge rows.
   Recommend (b): it is the owner's measured choice, and KH2's own offset is computed from a portrait
   of a different size; (a) is the 1:1 alternative if you'd rather.
3. **The slide-in** (KH2's bar rises in when the HUD is made, groups 21 / 50): (a) left out (the owner,
   s321: "I dont need the animation of the drive gauge rising up"); (b) KH2's; (c) a row. Recommend (a).
4. **KH1's MP HP label**: (a) hidden while the gauge shows (s320 / s322 "remove it"), done once as data
   (the label's records given a size of 0 or KH1's plain plate, so the MP bar's bottom edge stays, the
   s324 problem) instead of main's per-frame swap; (b) kept, the gauge placed under it; (c) a row.
   Recommend (a). First the label's record has to be identified in game (a one-time log of Sora's
   frame sprites), since the layout comes from a file.
5. **When it shows**: KH2 has no form-owned rule (1.5). (a) whenever KH1's gauge shows (KH2's rule);
   (b) only once a form is owned (s323 "don't show the drive gauge until the first drive form is
   unlocked", read from the owned byte `+0x881`); (c) (b) only with Progression > Story Unlocks on.
   Recommend (b): with Story Unlocks off every form is owned from the start, so it plays as (a); on, it
   keeps s323.
6. **Beyond KH1's own HUD rules** (s326: "only showing when the character is actively controlling
   sora"): KH1 already hides its gauge in cutscenes and off the field. (a) follow KH1's gauge only,
   then check in game what's left (talks, scripted walks, the pause menu); (b) also hide while KH1's
   event-running word `0x14298C9A0` is set (one read a frame); (c) main's whole list. Recommend (a):
   native, no per-frame guess; anything left gets its own small version with a KH1 signal.
7. **Config rows** (testing's Drive submenu): (a) one row, Gauge HUD On | Off (On by default; the label
   follows it); (b) also a label row; (c) none (always on). Recommend (a). Main's ini keys
   `drive_gauge_art`, `drive_gauge`, `drive_gauge_scale`, `hide_hp_label`, `drive_gauge_debug` are not
   carried over.
8. **KH2's sounds 0x20 / 0x21** (missing from main): (a) KH2's, from KH2's system bank through the
   sound cache (KH1 has no sound for these, the owner's rule for menus); (b) none. Recommend (a)
   (whether the sound cache holds KH2's system bank: Assumption, to check).

Write "go ahead" with the answers.

**The owner's answers (2026-10-29):** 1: asked what "KH2's art" means (below). 2 (b) main's s322 place and
size. 3: **the slide-in comes later** (the owner: "take a note, we'll add this later"). 4 (a), **and it needs
more work**: KH1's "MP HP" label must come back whenever the gauge isn't shown, e.g. with Form Cost on MP (the
owner: "it should appear when a setting disables the drive form though e.g: drives cost mp not drive gauge").
5 (b). 6 (a). 7: no row of its own; the gauge follows the Drive settings (the owner: "this gets enabled when
you set drive forms On then the drive gauge gets disabled if drive forms are set to drive gauge or mp";
restated to confirm below). 8 (a).

**Question 1, restated.** The gauge is always drawn with KH2's own art when it can be: the `gaug` sheet from
the player's own `zz0field.2dd` and its HD image, converted once into the mod folder. The question is only what
happens if that can't be built (KH2's files missing or unreadable): (a) no gauge, and a log line saying why;
(b) a plainer gauge drawn from KH1's own HUD pieces instead. Recommend (a).

**Question 7, restated to confirm.** The gauge shows when Drive is on (the second command page with Drive,
ini `drive`) and Drive > Form Cost is Drive Gauge; with Form Cost on MP (or Drive off) it isn't drawn and KH1's
"MP HP" label is back.

**Answer 1 (2026-10-29):** (a), KH2's art only (the owner wrote "7. (a)" under question 1's clarification;
taken as question 1's (a), and question 7 as confirmed: the gauge follows Drive and Form Cost). "go ahead".

**Built:** t936 KH2's gauge art built once into the mod folder (`hudart`, `kh2hud`). Next: t937 the texture in
KH1, t938 the draw (with the "MP HP" label hidden while the gauge shows, back with Form Cost MP), t939 KH2's
animations and sounds; the slide-in later (the owner).

### Round 4: Drive orbs as KH1 prizes (row 3), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §15. KH2: kinds 7 / 8 are its Drive orbs (1 and 3 points; in a form
12 ticks a point; Master Form EXP), dropped by each enemy's `przt` counts × Jackpot, munny turned into orbs by
Drive Converter. KH1: types 0..6, a seven-row table read at seven places. Questions in chat: the drop rule for
KH1's enemies (no `przt` id), the art, the pickup's place. Waiting for "go ahead".

**Answers (2026-10-30):** 1 (c), 2 (a), 3 (a), "go ahead". **Built:** t944 the types and their look (MP's orbs in gold), t945 the
drop (KH2's 94 normal-enemy rows), t946 the pickup (1 / 3 points; 12 ticks a point in a form). Answer 3 (a) "(a)": t947 KH2's orb shape and texture added to KH1's prize file, t948 that file built once into the mod folder and handed to KH1's prize effect (MP's orbs in gold as the fallback). **Tested (owner, 2026-10-30, t948 log):** KH2's orbs drawn in game; a drop of KH2's row (0, 2) × KH1's count factor 2 gave 4 big orbs, each +36 form gauge ticks in Wisdom (3 × 12, KH2's). Row 3 done.

### Round 5: Drive Recovery items (row 4), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §16. KH2: `0x112` / `0x113`, +3 / +9 bars (value × 100 points), usable
from the menu only while the bars are below the maximum, never sold. KH1: the battle item use is found; the pause
menu's use isn't decompiled (to read from the exe). Questions in chat: where they come from, battle use, the ids.
Waiting for "go ahead".

**Answers (2026-10-30):** 1 (a), 2 (b), 3 (a) "but if its in use, then use (b)" (not in use), "go ahead". **Built:** t949 the two items (records, names, help, menu use, Config: Drive > Recovery in Battle); t950 the item shops (Config: Progression > Drive Recovery); t951 the enemy drops; t952 the chests. Row 4 built; to test.

### Round 6: Drive support abilities (row 5), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §17. KH2: Damage Drive (50 × damage / max HP a copy, out of a form),
Form Boost (drain × 0.8 a copy), Drive Converter (munny × 0.5 a copy kept, the rest Drive orbs), Damage Control
(found after the round: half damage while HP ≤ max × 0.25 a copy). Questions in chat: Damage Control, Drive
Boost, the Drive Converter's place.

**Answers (2026-10-30):** 1 (a), 2 (a) "add it for after we implement the main build", 3 (a); "go ahead". **Built:** t953 the
four in the KH2 list, t954 Damage Drive, t955 Form Boost, t956 Drive Converter, t957 Damage Control. Drive Boost: after the
main build.

### Round 7: Partners leave during a form (row 11), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §18. KH2: Valor takes Goofy, Wisdom Donald, Limit no one, Master / Final / Anti
both slots (guests too); 15-frame fades; back near Sora on the revert. KH1: its own "member out" op `0x1402BB310`
(flags, HUD slot hidden). Open: events that command a partner who is out.

**Answers (2026-10-30):** 1 (a), 2 (a), + a Drive toggle for the party being required; "go ahead". **Built:** t958 Drive > Party
Required; t959 the partners leaving and coming back. Row 11 built; to test.

### Round 8: Forced reverts (row 6), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §19. KH2: room jumps carry a recovery kind (revert + bars refilled, some HP / MP too),
events can revert by script (no refill), battle end and KO revert nothing (KO = game over). KH1 has no recovery kinds.
Questions in chat (scenes vs row 11's answer, battle end, the Coliseum, KO).

**Answers (2026-10-30):** 1 (a), 2 (a), 3 (a), 4 (b); "go ahead". Row 6 done by t934 and t959; **built** t961 for row 7 (the
cup's start reverts, rounds carry the form, no drain in cutscenes).

### Round 9: Talking / examining and KH1's limits in a form (rows 18, 19), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §20. Row 10 is done by t958 (Drive > Party Required). KH2: Talk, Examine, Open and Save
have DisableForm 0 (allowed in every form); KH2's limits are party limits, so none in a form (Assumption: no other form check).
KH1: Talk is command 4, chest Open 0x37, object commands on the default path; testing's Valor refusal blocks all of them and
lets the limit ids through (their hits refused later); Wisdom blocks none. Questions in chat.

**Answers (2026-10-30):** 1 (a), 2 (b); "go ahead". **Built:** t962 Valor lets talking, examining, chests and object commands through;
t963 Drive > Limits in Forms. Rows 18 and 19 built; to test.

### Round 10: The dim during a Drive change (row 21), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §21. KH2: the change multiplies the scene's light and fog colour by 48 / 128 (in 12
frames, held, out in 15), plus a second screen effect not looked up. KH1: its light list (`0x142D5B030`) with its own fade,
which KH1 already uses to dim its field. Questions in chat (fog, the second effect).

**Answers (2026-10-30):** 1 (a), 2 (a); "go ahead". **Built:** t964 the dim (light and fog). KH2's second effect is a frame feedback blur
KH1 has no draw for: question in chat.

**Blur answer (2026-10-30):** (a); "go ahead". KH1 turned out to have the same blur (`0x140185C40`): **built** t965 on it, no renderer change.
Row 21 built; to test.

### Round 11: The second Keyblade's slot (row 20), 2026-10-30

Notes: `docs/kh1-drive-forms-native.md` §22. KH2: a form's empty slot uses Sora's own Keyblade (`0x1403DFEB0` case 0x23B). KH1:
its free count `0x1402E8770` and compare box `0x1403050D0` (Steam). Conflict: main's s306 Dream Sword default vs KH2's rule and
testing's "Dream Sword isn't a Keyblade". Questions in chat.

**Answers (2026-10-30):** 1 (a), 2 check the decomp (KH2 moves the copy out of its stock; KH1's free count subtracts it here); "go ahead".
**Built:** t966 empty slot = Sora's own; t967 slotted Keyblade in use; t968 compare box. Row 20 built for Valor; to test.

### Round 12: Limit, Master, Final and Anti Forms (rows 12 to 15), 2026-10-30

The owner: "do what you were investigating, bring in final from the main branch but make sure you check against the c decomp".
Notes: `docs/kh1-forms-research-r12.md` (one section per form: testing now, KH2 checked, the native KH1 path, version plans,
questions). Proposed order: Limit, Master, Final, Anti (Anti's roll also gives Final). Questions in chat.

**Answers (2026-10-30):** 1 (a) KH2's limits, KH2's animations / effects / hitboxes, a Trinity toggle, a Limit sub menu in Drive;
2 (c); 3 (b); 4 (a) usable under the cost, MP Charge later; 5 (a); 6 (a) noted; 7 (b); 8 (b); 9 (a), (b) to fall back to (noted).
**Built:** t969 Limit Form's moves on KH1's own states. Row 12 under way.
**Built (cont.):** t970 compile fix; t971 Limit Form's effects and sounds (`kh2dpx::Pkg::Limit`); t972 Config > Drive > Limit (Cost,
Limits in Forms moved in). Next for row 12: the look (the owner's black outfit, our own texture ops built once), the level record
(save bytes to scan), the Limit list in Magic's place, the limits (Sonic Rave first), HP drain, the under-cost rule, Trinity.
