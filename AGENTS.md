### 2026-10-29: Stage 936 (testing branch: KH2's Drive gauge art built once into the mod folder)
- The owner (chat, the main-branch port's round 3 for the Drive gauge HUD: answer 1 (a) "KH2's art only" (no gauge and a log line without it), the tester's rule "the game's assets are built once, kept in the mod folder"; "go ahead").
  - KH2's art (Checked (files)): `field2d/<lang>/zz0field.2dd` is a BAR of images (type 24) and sequences (type 25) `fiel`, `gaug`, `mapj`, `miss`; the Drive gauge is the `gaug` SEQD (188 456 bytes) on the second image, whose HD picture is the second remastered PNG (1024 × 512, 8-bit RGBA). KH2 draws it from its player gauge object (`0x140181270` / `0x1401816C0`, Checked (decomp), §14).
  - New `hudart.rs` (pure, tested): the BAR reader, the `gaug` sequence and its image index, our own PNG decoder (zlib inflate and the five row filters, 8-bit RGB / RGBA; nothing new linked in), the cache file `kh2-hud-gauge.bin` in `kh1-kh2-cache` (the slot cache's layout: a manifest with this format, the mod's version, the game build, `zz0field.2dd`'s PKG entry and the picture's size; two blocks, the SEQD and the picture as B8G8R8A8). Checked here on the player's own files: the decoder's pixels equal Python's zlib decode of `US_zz0field_2dd1.png` byte for byte (10 ms).
  - New `win/kh2hud.rs`: at start, with Drive on, a thread finds KH2's install (the Mods Manager's backup first, as the sound cache), keys the cache by `zz0field.2dd`'s PKG entry (`stamp_any`: nothing read when it matches), else reads the BAR and its HD pictures from the PKG, builds and writes the cache; the art kept for the draw (`kh2hud::art`, a later version). Log (Loading): "Drive gauge art: ...". Nothing drawn yet; nothing on the game's thread.
  - Assumption: the Steam PKG's remastered pictures for `zz0field.2dd` are PNGs (as the extracted files are; if not, the log says so and nothing is drawn). `hudart` tests run with rustc here (11 with `slotcache`, all pass); `cargo test` for `kh1-hook` not run (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §14.

### 2026-10-29: Stage 935 (testing branch: Progression > Drive Gauge Growth: the owner's six-fight maximum)
- The owner (chat, round 2 answers 2 (b) "but we'll add bonus levels later", 3 (a) its own Progression row; main s325, s326: "Let's have the drive gauge start at 3. Have the drive gauge level up at these points: ... Sabor ... Riku I ... Sephiroth ... Kurt Zisa ... Unknown", "instead of beating the game, make it after defeating Chernabog"; "go ahead").
  - KH2 (Checked (decomp)): the maximum rises only by bonus levels (`0x1403EDF90`, row `+4`, cap 9), a full gauge staying full. KH1 has no bonus levels (to come later, the owner), so the six fights are the owner's rule.
  - New Config row Progression > Drive Gauge Growth (Off | On, ini `gauge_growth`, off by default; `drivegauge::growth` / `set_growth`, live). On: `drivegauge::max` = 3 + `storyforms::gauge_fights` (pure, tested: Deep Jungle's story byte >= 0x42 Sabor, Hollow Bastion's >= 0x32 Riku 1, End of the World's >= 0x33 Chernabog, journal 0xA5 Sephiroth, Agrabah's >= 0x96 Kurt Zisa, journal 0xA9 with item 0x3C the Unknown; main's flags, Assumption on testing), at most 9. Off: 9.
  - The save's points 100 now read as a full gauge at whatever the maximum is (KH2's full gauge stays full as the maximum rises); bars over a lowered maximum are cut to it. `drivegauge` / `storyforms` tests run with rustc here (17 pass); `cargo test` for `kh1-hook` not run (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §13.

### 2026-10-29: Stage 934 (testing branch: Drive > Gauge Refill: KH2's refill on leaving a world in a form)
- The owner (chat, round 2 answer 4 (a) KH2's refills, "there's also an exploit where if you leave a world while being in a drive form your drive gauge gets max refill", then "just have it be off or kh2"; "go ahead").
  - KH2 (Checked (decomp), Steam KH2): the only full refill is `0x1403C2020` (bars to the maximum, points 100, the form time 0), from `0x1403A5790`, asked for by the recovery request `0x1403A5650`; entering the world map (`0x1403FC350` case 0xC) asks for kind 1, which in a form or a summon reverts and refills the gauge, HP and MP (out of one it refills only HP and MP). Map jumps, area programs and a Continue-type menu ask for other kinds (their triggers are data: Assumption, not ported). No save point or level-up refill in code.
  - New Config row Drive > Gauge Refill (Off | KH2, ini `drive_refill`, `kh2` by default; `form::gauge_refill` / `set_gauge_refill`, live). KH2: `form::native_frame` notes the world the form was entered in (`takeover::current_world`, KH1's world word, the address the room log reads); a form running in another world (KH1's world map lies between them) gets the gauge refilled (`drivegauge::refill`), HP and MP full (KH1's own HP and MP adds) and the revert. Off: nothing (the form carries on, as before t934).
  - Assumption: KH1's world word changes only when Sora goes through the world map (not checked for every world's cutscene warps). `cargo test` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §13.

### 2026-10-29: Stage 933 (testing branch: Drive > Form Cost: KH2's Drive bars and form gauge, or MP)
- The owner (chat, round 2 answers 1 (b) the Drive gauge by default, 8 (a) KH2's way: bars to enter, MP untouched, the form ends with its form gauge; "go ahead").
  - KH2 (Checked (decomp), Steam KH2): the Drive command is usable while its cost (`cmd +0x1C`) is within the full bars (`0x140401C80`); choosing pays it (`0x1403C2780`); entering (`0x1403C17C0`) sets the form gauge to (form level + 3) × sstm `+0x68` (600) ticks; the player step (`0x1403A92C0`) drains one tick a frame in battle, sstm `+0x6C` (0.5) outside; a hit in the form takes its drive value × sstm `+0x74` (15) ticks, × sstm `+0x64` (0.2) on a normal enemy; empty, the form's think (`0x140404A40`) reverts, the bars untouched. Numbers Checked (files), 03system.
  - New Config row Drive > Form Cost (MP | Drive Gauge, ini `drive_cost`, `gauge` by default; `form::gauge_cost` / `set_gauge_cost`, live; a form under way keeps what it was paid with). Drive Gauge: `form::can_pay` checks KH2's cost (`drivegauge::cost`: Valor 3, Wisdom 3) against the bars (the Drive list greys by it, `usable` / `possible` / `choose`, and the change's own check in `native_start`); `native_start` pays the bars (no MP); `native_arrive` gives the form its form gauge from its form level (`formexp::record`, Lv1 the first time); `drain_step` drains it and the hits' losses (`form::hit_drain`, from `drivegauge::hit` at KH1's contact, applied at the next step), no MP drain; empty: the revert (logged "the form gauge out"). MP: as before (3 MP, the MP drain, out of MP).
  - KH1 has no MP Charge and no form EXP mode change for the gauge; Form Boost (×0.8 a copy) waits for the support abilities (row 5). Assumption: KH2's no-drain-while-held flag (`0x142A11478`) has no KH1 counterpart wired (the drain follows `native_frame`'s own pauses). The gauge isn't drawn yet (row 2): the log shows it. `cargo test` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §13.

### 2026-10-29: Stage 932 (testing branch: the Drive gauge filled by Sora's hits at KH1's contact)
- The owner (chat, round 2 answers 6 (a) KH1's own hits by KH1's attack data, 7 (a) KH1's boss signal, all enemies normal until it is found; "go ahead").
  - KH2 (Checked (decomp), Steam KH2): a hit's Drive is the ATKP row's byte `+0x29` (the attack owner's hit callbacks `0x1403A7480` / `0x1403A8800`), only on an enemy (class bit `+0x6C8` 0x10), × sstm `+0x60` 0.75 on a normal (objentry type 4) enemy, then `0x1403A9CF0`: × MP Charge's factor while MP charges, × the world's factor, truncated, into the add `0x1403C0AD0`. In a form (mode 2) the hit drains the form gauge instead (t933).
  - `drivegauge::hit` at KH1's contact (`probe::hit`, `0x1402BF330`, hooked since stage 9; no new hook): Sora's hit record on a KH1 actor of kind 3 (an enemy) adds KH2's drive value (`kh1rows::kh2_atkp`, the KH1 attack object's KH2 row) or, for KH1's own moves, KH1's attack data `+0x2C` (what KH1's contact charges MP with, Checked (decomp)), × 0.75, truncated. KH1 has no MP Charge; the world factor 1.0 (Assumption: KH2's default not found). In a form or changing it goes to `form::hit_drain` (empty until t933).
  - Assumption: every KH1 enemy is a normal one (KH1's boss signal not found yet, answer 7). Assumption: KH1's attack data `+0x2C` is 3 on swings, 6 on finishers, 0 on spells (research round, not traced in game). `cargo test` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §13.

### 2026-10-29: Stage 931 (testing branch: the Drive gauge's data in Sora's save)
- The owner (chat, the main-branch port's round 2 for the Drive gauge: answers 2 (b) the maximum 9 with the six-fight row off, "we'll add bonus levels later"; 9 (a) main's save bytes; "go ahead").
  - KH2 (Checked (decomp), Steam KH2): the gauge is full bars, points 0..100 and a maximum in the player's status (`+0x1B0..+0x1B2`); a new game starts full (`0x1403A0E70`); the add `0x1403C0AD0` carries points into bars under the maximum and holds at 100 there, a loss breaks a bar; the pay `0x1403C2780` floors bars at 0 and clears the points when the gauge was full or short; the maximum rises only by bonus levels (`0x1403EDF90`, cap 9), a full gauge staying full. Costs from KH2's 03system `cmd` (`+0x1C`, Checked (files)): Valor 3, Wisdom 3, Limit 4, Master 4, Final 5, Anti 9.
  - New `drivegauge.rs` (pure, tested): those rules, the form gauge's numbers (sstm `+0x60` 0.75, `+0x64` 0.2, `+0x68` 600, `+0x6C` 0.5, `+0x74` 15, Checked (files)) and the save format. New `win/drivegauge.rs`: the gauge in Sora's save, bars at `+0x37` (bit 7 written, unwritten reads full), points at `+0xAB` (main's bytes, answer 9 (a)); the maximum 9 (answer 2 (b); bonus levels later). `get`, `bars`, `add`, `pay`, `refill`, each logged when it changes the gauge.
  - Save bytes: the spare byte after the weapon in Sora's and Donald's party records, in KH1's save field list; no function of the Steam exe that loads the save pointer `0x142868BA0` touches `+0x37` / `+0xAB` (Checked (exe), objdump scan); testing used neither (Checked (code)). Assumption: nothing reaches them through a pointer from elsewhere.
  - Nothing calls it yet; play unchanged. `drivegauge` tests run with rustc here (16 with `formlevel` / `storyforms`, all pass); `cargo test` for `kh1-hook` not run (no crate registry reachable). docs/kh1-drive-forms-native.md §13.

### 2026-10-29: Stage 930 (testing branch: Valor's Keyblade slot filled when Valor is gained)
- The owner (chat, round 1 answer 5 (a): fill the slot at gain, keep testing's refusal; "go ahead").
  - KH2 (Checked (decomp), Steam KH2): adding a form item (`0x1403C3FE0`, kind 0x15) puts, for Valor, Master and Final, the first owned Keyblade in item-table order (kind 2 records) into the form's empty Keyblade slot (record `+0`) and takes it out of the inventory; Valor itself is never refused for a missing Keyblade (`0x140401C80`).
  - `formown::valor_slot`: when Progression > Story Unlocks gives Valor (silently or during play, t926 / t929), Valor's slot (`equip::VALOR_OFF`, save `+0x488`), if empty, gets the first Keyblade with a copy not equipped (`equip::valor_candidates`, lowest id first, KH1's item order); none: it stays empty. KH1's free count already leaves that copy to Valor (no inventory change). Testing's refusal of Valor without a spare Keyblade stays (`form::own_needs`), as answered (a conflict with KH2, kept for the two-blade moveset). With Story Unlocks off nothing changes (Valor isn't gained, he always has it).
  - `cargo test` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §12.

### 2026-10-29: Stage 929 (testing branch: the "New Drive Form" box)
- The owner (chat, round 1: main s308 "the game should tell you when you get a drive form"; answers 4 (a) and 6 (a); "go ahead").
  - `formown::gained`: a form the owner's table gives during play (Progression > Story Unlocks on, not the save's first read or the pass right after the row is switched on, t926) queues KH1's prompt box "New Drive Form" / "<form name>" (`menu::queue_prompt`, the same box as "Form Level Up" and "New Ability": KH1's level-up box record at Steam `0x14283B390`, Checked (decomp)). Only for the forms in the Drive list (Valor, Wisdom); Limit and Master (and Final, on hold) are stored and get their box when they are built (answer 4 (a)).
  - KH2 (Checked (decomp)): the item add `0x1403C3FE0` shows nothing for a form item; a form given as a bonus is a line in the bonus window (`0x1403EDCA0` -> `0x1403EDB20`). The box is the owner's (main s308), not KH2's.
  - `cargo test` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §12.

### 2026-10-29: Stage 928 (testing branch: the Drive list from the owned forms, KH2's greyed "?" with none)
- The owner (chat, round 1: "go ahead"; answer 7 (b) KH2's placeholder).
  - KH2 (Checked (decomp), Steam KH2): the Drive submenu (`0x1403F3110`, and inline in the command menu builder `0x1403F29E0`) lists only the forms whose item is owned, in the order Valor, Wisdom, Limit, Master, Final, Anti; with none listed the Drive slot holds command 0x68, whose 03system `cmd` row (kind 0, message 0x8503) reads "?" in `msg/us/sys.bar` (Checked (files)), greyed by the state function `0x140401D60` (kind 0, Checked (decomp)).
  - `drive.rs`: the Drive list (menu 3) holds the owned forms only (`formown::owns`, t925 / t926), in `FORMS`'s order (Valor, Wisdom: KH2's); greyed as before when one can't be chosen now. With none owned the Drive row (row 4 or page 2's) is named "?" and greyed (no form usable, so it doesn't open), the list left alone. In a form every entry stays (they read Revert). A form not owned is refused at `chosen`. The DRIVES title and colours follow any form record in the list's first entry (was Valor's only).
  - With Progression > Story Unlocks off (the default) Valor and Wisdom are owned, so nothing changes; on, the list follows the owner's table. Assumption: `menu::encode`'s "?" (0x60) is KH1's question mark (as used elsewhere). `cargo test` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §12.

### 2026-10-29: Stage 927 (testing branch: Top > Drive, its MP Refill row: KH2's MP refill on entering a form)
- The owner (chat, round 1: "same for drives" (a Drive submenu in the Config menu's main list) and answer 8 "(a) but make it a toggle in drive config menu"; "go ahead").
  - KH2 (Checked (decomp), Steam KH2): entering a form (`0x1403C17C0`, from the change task `0x14040CC00`) sets HP and MP to their maximum (MP Charge cleared), then the form gauge. Testing refilled HP only (`form::heal_full`).
  - New Config list Top > Drive (before Progression; `config::List::Drive`), its first row MP Refill (Off | On, `Row::DriveMpRefill`, ini `drive_mp_refill`, off by default so nothing changes until it is switched; `form::mp_refill` / `set_mp_refill`, live). On: `form::native_start` refills MP to max right after HP, through KH1's own MP add (Steam `0x1402A4A00 (actor, n)`: stats `+0x44` + n within 0..`+0x48`, the HUD's copy updated, Checked (decomp); its first 12 bytes checked against the exe, refused on a mismatch, Checked (exe)); Steam only.
  - Note: with the forms still paid and drained in MP (testing's cost), a refill on entry gives the form a full MP bar to drain; KH2 pays in Drive bars. Assumption: KH1's MP add scales a gain for an actor with flag `+0x88 & 0x20` unless ability 9 is equipped (the decomp's branch; not traced). The refill runs at the Drive motion's start with the HP refill, KH2's at the arrival (`0x14040CC00`).
  - `cargo test` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §12.

### 2026-10-29: Stage 926 (testing branch: Progression > Story Unlocks: the owner's Drive Form table, off by default)
- The owner (chat, round 1 answers: 1 (a) off = every form testing has, 2 (b) off by default, 3 "for the progression make a new sub menu in the main part called progression", 4 (a), 6 (a); "go ahead").
  - New Config list Top > Progression (after Command Menu; `config::List::Progression`), its first row Story Unlocks (Off | On, `Row::StoryUnlocks`, ini `story_unlocks`, off by default; `formown::story_unlocks` / `set_story_unlocks`, live).
  - New `storyforms.rs` (pure, tested): the owner's table from the main branch (s279, s307; its own module here, nothing of main's code): Guard Armor (Traverse Town's story byte `+0x1504` >= 0x2B), Opposite Armor (journal entry 0x9F, `+0x16E3` high bit first), the Pegasus Cup (`+0xF37` >= 1) give the first, second and third form by the Dive's choice (`+0x48C`: Sword Valor, Limit, Wisdom; Shield Limit, Wisdom, Valor; Rod Wisdom, Valor, Limit); Anti-Sora (`+0x150D` >= 0x32) Master; Riku 2 (`+0x150E` >= 0x82) Final. The offsets: KH1 story bytes `0x1401B6B00`, journal `0x1401B5980` / `0x1401B6640`, Dive choice written by `0x1401B0380` and read by the level-up `0x1402B51B0` (Checked (decomp), Steam KH1); the thresholds, the journal id, the cup byte and the Dive values are main's (Assumption).
  - `formown`: Story Unlocks on, `check` (once a second) gives the table's forms in the save byte (`+0x881`); its first read on a save (new bit 6) or the pass right after the row is switched on gives them silently (answer 6 (a)); later ones are gains (logged, their box in a later version). Forms not built (Limit, Master) are stored and wait (answer 4 (a)). `owned`: off = Valor and Wisdom (and Final once entered), as before; on = the forms in the save. Only the form level cap reads it so far.
  - `storyforms` tests and `formlevel`'s run with rustc here (12 passed); `cargo test` for `kh1-hook` not run (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §12.

### 2026-10-29: Stage 925 (testing branch: the owned Drive Forms in Sora's save; the form level cap from them)
- The owner (chat, the main-branch port's round 1 for the story flags: "go ahead"; docs/main-port-inventory.md round 1).
  - KH2 (Checked (decomp), Steam KH2): owning a form is owning its form item (`0x1403C3EF0` the form's item, table `0x1405C5D08`; `0x1403C4720` owned); the form level cap is 2 + the forms owned among Valor, Wisdom, Limit, Master and Final (`0x1403E8380`, `0x1403E88B0`). Testing had no owned state and counted Valor and Wisdom as always owned.
  - New `formown.rs`: the owned forms are a byte of Sora's save, `+0x881` (no Steam instruction touches `+0x881..+0x887`, Checked (exe) in `formexp.rs`; unused by us, Checked (code)): bits by KH2's form id − 1 (`formlevel::owned`), bit 7 "set up". `formown::check` (in `menu::Growth::update`'s once-a-second pass, every abilities mode) sets a save up once, silently, with Final's bit if its form record is set. `formown::owned` = Valor and Wisdom (as before) and the forms given in the save; entering a form gives it (`formexp::entered`). The level cap reads it (`formexp::cap`: 2 + `formlevel::owned::for_cap`, Anti not counted, Limit counted); with Valor and Wisdom it is 4, Final entered 5, as before.
  - Nothing else reads it yet; play unchanged. `formlevel` test `owned_forms_for_the_cap` (run with rustc here: 8 passed). `cargo test` for `kh1-hook` not run here (no crate registry reachable). Not checked in game. docs/kh1-drive-forms-native.md §12.

### 2026-10-29: Stage 924 (testing branch: ini keys that did nothing removed)
- The owner (chat, the main-branch port: "any ini setting that does nothing in the testing branch should be removed", "go ahead").
  - Removed from the ini (`settings::DEFAULT_INI`, the `Settings` record and `load`): `carry`, `ceilings`, `grabs`, `swing_fx` (parsed, never read since the KH2 controller's deletion, t901), `air_cast_fall` (parsed, never read; `AIR_CAST_FALL` with it), the `glide` line (not read since t319: `glide_on_square` stays, always off, for the held-off Square glide code), and the `air_magic` read (no Config row; `styles::Kind::AirMagic` is now always KH1). Checked (code): no reader of those fields outside `settings.rs`; every KH1 air cast request (`kh2slots::CASTS`, 0x83..0x89) takes its style from its spell's row (`cast_spell`, `formmagic::anim_kh2`), never from `AirMagic`. Every other key in the default ini has a reader (Checked (code), a key-by-key search).
  - An ini that still has those lines keeps them (a save copies unknown lines), they're just not read. `openkh/README.txt` loses its two `glide = square / jump` mentions. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/main-port-inventory.md (Ini).

### 2026-10-29: Stage 923 (testing branch: long Config menu texts drawn smaller to fit their boxes)
- The owner (chat: "config menus texts like kh1 ability chance (leaves the backdrop on the last c in chance) and knock-back numbers (leaves in-between b and e on numbers) go off the backdrop and the text needs to be scaled down to fit it and any other long winded config text").
  - KH1's panel call (Steam `0x1402F3950`) draws a label left-aligned from x 27 in a box spanning x 11..153, and a value centred in its box, both at KH1's panel text size (`0x140545364`, 20); KH1's text draw (`0x1402EBCD0`) and its width function (`0x1402D0E10`) both read a size code `0C nn` inside the text, and the draw a line move `0E lo hi` (Checked (decomp)).
  - `config::fit`: each of our label and value texts is measured once with KH1's own width function at that size; one that is wider than its room (label: 118, to 8 short of the box's right edge; values: the box width less 16) gets a copy that starts with KH1's size code at the largest size that measures inside the room and a move down of half the size it lost, so it stays centred in the row (the draw's glyphs are size x 16 tall from the line's top, Checked (decomp)). The panel is handed that copy's id (`0x7F40xxxx`, through our text lookup); texts that fit and KH1's own are left alone. Log (Menus): "text ... is N wide for 118: drawn at size S of 20".
  - Assumption: KH1's own widest labels end about 8 short of the box's edge (the room). Not checked in game. docs/kh1-config-menu.md §7.

### 2026-10-29: Stage 922 (testing branch: ini `experimental`, the experimental Config rows and their ini lines)
- The owner (chat: "a setting in the ini that enables experimental settings in the config menu and pop up at the bottom of the ini once enabled": Enemy Weight (On), Clash Combo (On), Knock-back Numbers (Scaled), Medium Hits (Blow), KH2 Damage (Off), Recoil (KH1), Guard Shape (KH1); changeable while enabled, back to those defaults when disabled again).
  - New ini key `experimental` (off by default, after `config_row`). Off: Combat > Tweaks hides Enemy Weight, Clash Combo, Knock-back Numbers, Medium Hits, KH2 Damage and Recoil, and Combat hides Guard Shape (`config::TWEAKS_STABLE` / `COMBAT_STABLE`); `settings::load` gives each the default above whatever the ini says, and takes their lines (and the block) out of the ini, with each key's comment lines (an ini from before t922 has them in its body).
  - On: the block "experimental settings" with those seven keys at those defaults is put at the bottom of the ini once (`settings::experimental_ini`); after that it stays there with whatever is set in it (the Config rows write into it), kept at the bottom if a later save adds a line after it.
  - Switched while the game runs: on the reload after the ini's save the seven live values are read again (`knockback::forget_weight`, `recoil::forget_clash_count`, `hitreact::forget`, `kh2damage::forget_kh2_damage`, `styles::reload` for Recoil and Guard Shape), and an open Combat or Tweaks list is made again with its new rows.
  - Defaults changed with it: `kh2_damage` is off and `knockback_numbers` scaled when the ini doesn't say (were on and kh2). Not checked in game. docs/kh1-config-menu.md §7.

### 2026-10-29: Stage 921 (testing branch: Guard Animation and Ledge Hop in Tweaks, "Air Base Combo")
- The owner (chat: "put guard animation and ledge hop in tweaks", "change aerial base combo to air base combo").
  - Combat: Physics, Base Combo, Air Base Combo, Guard Shape, Magic, Tweaks. Tweaks: KH1 Ability Chance, Enemy Weight, Cast from Glide, Clash Combo, Ledge Hop, Guard Animation, Hit Reactions, Knock-back Numbers, Medium Hits, KH2 Damage, Criticals, Party Criticals, Recoil. Each row keeps its setting, ini key and help; Tweaks' help names the two.
  - "Aerial Base Combo" is "Air Base Combo" in the Config menu (label and help), the move-style log and the ini's comment (`air_combo` keeps its key). Not checked in game. docs/kh1-config-menu.md §7.

### 2026-10-29: Stage 920 (testing branch: KH2's spells deal KH1's own spell hit, KH2's power only the proportions)
- The owner (chat: KH2 Blizzard felt like a worse KH1 Blizzard after t919; "go ahead" on KH1's own magic formula with KH2 setting only the proportions; Explosion and the other Magic-based Keyblade moves stay on Ripple Drive's base).
  - Before (t919): KH2 Fire / Blizzard / Thunder hits took Ripple Drive's magic stat (10 + max MP x 3) x KH2's power % (Blizzaga 375%): about 15 a cast on the Coliseum foes of the log (Assumption: Defense 27, max MP 7).
  - Now: a KH2 spell hit on KH1's own spell data keeps that data whole (KH1's power points + max MP, `bodyhits` spell objects and `combat`'s spell path as before t919; `magic_source` removed), and `kh2damage` counts KH2's power against KH2's normal cast at that KH1 level (`kh1_spell_level`: the data matched against KH1's spell entries; `normal_cast`: KH2 Blizzard ATKP 248 subs 0 / 2 / 4, 250 / 300 / 375%, Checked (files)) instead of 100. A normal cast deals KH1's own spell hit (Blizzaga 24 there), the finisher 550 / 375 of it, Firaga's ticks 50 / 375 each. Ripple Drive's base stays for Magic-based hits without KH1 spell data (Explosion, Magnet Burst, t917). Log: "KH1 spell level 3: of 375%".
  - Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-kh2-attack-data.md §13.

### 2026-10-29: Stage 919 (testing branch: KH2's spells on KH1's magic stat, KH2's power % their whole strength)
- The owner (chat: KH2 Blizzard did 90 a hit; "(b) go ahead": one KH1 magic base for every Magic-based KH2 attack, KH2's % the only multiplier).
  - Before: a KH2 Fire / Blizzard / Thunder hit landed on KH1's spell data at Sora's KH1 level (Blizzaga: power 34 + max MP on KH1's curve) and KH2's power (Blizzaga 375%, its finisher 550%, Checked (files) ATKP 248) multiplied that: KH1's spell-over-swing gap and KH2's on top (the log: 24 x 3.75 = 90, 24 x 5.5 = 132).
  - Now: every KH2 spell hit (ATKP stat source 1) takes KH1's magic stat on its data as Explosion's do since t917 (`kh1rows::kh1_magic`: Ripple Drive's stat source, max MP x 3, power 10, min / max), on a KH1 spell's data too, whose element (resistances) and reactions stay. `bodyhits` spell objects and `combat`'s spell path (`magic_row`; `magic_source` keeps the KH1 spell's effect list). KH2's level rows (250 / 300 / 375%, finishers 375 / 450 / 550%) are the spell's strength; KH1's spell level still picks the level that plays.
  - Assumption: KH1's 10 + max MP x 3 keeps pace with Sora's Strength as KH2's Magic does with Strength (not measured across KH1's levels). Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-kh2-attack-data.md §12.

### 2026-10-29: Stage 918 (testing branch: no KH1 finisher points under KH2's finisher power)
- The owner (chat: "the 'bonus on top' should not be in the game but work through kh1's systems. go ahead").
  - Before: a KH2 hit on a KH1 finisher row's data kept KH1's finisher power (4 or 6 points on KH1's damage curve's index, about x2 to x2.8) and KH2's finisher power % was applied on top.
  - Now (`kh2damage::damage`, Steam): for the one call of a KH2 hit whose curve is scaled, a Strength-based data (stat source 0, an element that reads the curve) has a plain swing's power (Sora's KH1 row 13's data `+0x48`, 0 in `btltbl.bin`, Checked (files)), put back after the call as the curve pointer is. KH2's power % is the finisher's whole boost. KH1's own hits, Magic-based hits (Ripple Drive's power, t917) and gravity / stop / cure / aero data untouched. Log: "KH1 row power 6 -> 0".
  - Assumption: KH1's damage function reads `+0x48` through the attack's data handle at the call. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-kh2-attack-data.md §11.

### 2026-10-29: Stage 917 (testing branch: KH2's Magic-based hits on KH1's own magic stat)
- The owner (chat: KH2 moves tied to Magic go through how KH1 works out magic damage, no bonus on top; answer (a), Ripple Drive; "go ahead").
  - Before: Magic-based KH2 hits (ATKP stat source 1: Explosion, Magnet Burst) landed on a KH1 swing's data (Strength) and were multiplied by (1 + 0.1 x max MP) x `magic_scale`.
  - Now: their data copies take KH1's Ripple Drive data's stat source, power and min / max (`kh1rows::kh1_magic`; Sora's KH1 row 7, data 0x4E88: max MP x 3, power 10, Checked (files); read live, refused if not a max MP source). KH1's constructor makes the stat term from max MP (Checked (decomp)); KH2's power % applies on KH1's curve as before. Our rows, body windows, left Keyblade windows, spell attacks on swing data, `land_hit`'s fallbacks (`combat::magic_row`). `combat::magic_factor` and the ini's `magic_scale` removed (also from openkh/README.txt).
  - Left: a Magic-based window on KH1's own swing row stays Strength (KH1's shared data; logged once). Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-kh2-attack-data.md §10.

### 2026-10-29: Stage 916 (testing branch: buses 5 and 6 follow the effects slider after a fade)
- The owner (chat: Olympus's wind came back after a Trinity with the effects slider at its lowest; "(b) go ahead": follow the effects slider after every fade, at every step).
  - KH1's fade (`0x1400DDD10`) sets buses 5 and 6 to the fade's own level (1.0 after a fade-in) through its two calls of the bus volume set (`0x1400547F0`, at `0x1400DDEA9` / `0x1400DDED3`), while the effects setter sets buses 4..8 to the effects step. Checked (decomp + exe). So after every room or event fade those two buses were at full, whatever the slider (vanilla too).
  - `volmute::bus_after_fade`: with ini `effects_after_fade = slider` (the default, read at start) those two calls go through ours, and a fade-in sets buses 5 and 6 to the effects step's volume from KH1's own table (0.0 at step 1 with Volume Mute), over the fade's own time; a fade-out passes through. Only runs when KH1 fades, never per frame; the call sites and KH1's settings getter are checked first. `vanilla`: not redirected.
  - Not checked: that the Olympus wind plays on bus 5 or 6 (Assumption, from the log: the room's sounds came back right after the Trinity's fade). Not checked in game. docs/kh1-volume.md.

### 2026-10-29: Stage 915 (testing branch: the lowest step of every volume slider is silence)
- The owner (chat: "when audio is set to the lowest setting it currently doesn't mute the game, can you make it so that the lowest setting on any of the audio sliders completely mutes that track?").
  - KH1 turns a slider step (1..10, the lowest the menu allows is 1) into a volume through its table at Steam `0x1403D8B60` (0.0, 0.1 .. 1.0), so step 1 was 0.1; its fade (`0x1400DDD10`, buses 1..3 back to their sliders after a fade-in) and a stream's start (`0x14011CEA0`, master; Assumption: the movie player) read the step as step / 10.0 instead. Checked (decomp + exe).
  - New `volmute.rs` (ini `volume_mute`, on by default, read at start): the table's step 1 becomes 0.0 and those four step / 10.0 reads become reads of the same table (same 19 bytes each, checked against a disassembler). All four sliders (master, bus 1, effects, voices) are silent at the lowest step; steps 2..10 unchanged. Every site's bytes are checked first, all or nothing; Steam only. Data and code KH1 runs itself, nothing per frame. Off: nothing patched.
  - Left as vanilla: buses 5 and 6 follow KH1's fade level (1.0 after a fade-in), not the effects slider. No Config menu row yet.
  - Not checked in game. docs/kh1-volume.md.

### 2026-10-29: Stage 914 (testing branch: Combat > Ledge Hop, KH2's drop-ahead hop as a Config row)
- The owner (chat: "lets make sora's drop ahead hop a toggle in the config menu under combat for now").
  - New Config row Combat > Ledge Hop (Off | On, after Guard Shape; ini `drop_hop`, on by default, so nothing changes until it is switched). On: KH2's line ahead (Steam KH2 `0x140402D20`, Checked (decomp)) marks row 1 at a drop ahead, the hop off the ledge in 181, as before. Off: base Sora's ground press doesn't test the line ahead (`combomods::Kh1Press::test_drop_ahead`), so row 1 is never marked and KH2's walk takes the next row that fits; Drive Forms keep KH2's. Live (`combomods::drop_hop` / `set_drop_hop`), kept in the ini.
  - Not checked in game. docs/kh1-anim-slots.md (the drop ahead, row 1).

### 2026-10-29: Stage 913 (testing branch: t911's build error)
- The owner (build output: `Mutex` not found, combat.rs `GRIP_CAL`).
  - `combat::GRIP_CAL` now names `std::sync::Mutex` in full (combat.rs imports no `Mutex`). Nothing else changed; the `chest.rs` `fetch_update` deprecation warning left as it is (unrelated).
  - `cargo test` not run for `kh1-hook` here.

### 2026-10-29: Stage 912 (testing branch: our hit state's enemy list only for the posed and fallback paths)
- The owner (chat: "do 1. and 2.", item 2).
  - Before: `combat::Strikes::look` (our list of KH1's bodies, guards and shots) ran every frame in `takeover::kh2_hits` and `takeover::native_spells`.
  - Now: `kh2_hits` runs it only when KH2's rig is posed (a window needs it, `combat::guard_needs_pose`); `native_spells` only when KH1's pass, KH1's barrier meeting or KH1's calls aren't all there. The guard repel's stopped attack is read from KH1's attack pool (hit mask bit, owner, place), not our shot list. Reflect's step runs the stopped-shot log check (once a game frame). Reflect's barrier shapes stay given by the spell script, as spell attacks.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-roll-guard-native.md §11.

### 2026-10-29: Stage 911 (testing branch: Guard Shape KH2, KH2's Keyblade guard shapes on KH1's grip)
- The owner (same request, item 1).
  - Before: with Guard Shape KH2, KH2's Keyblade guard shapes were KH1 guard objects we placed every frame on KH2's posed rig.
  - Now (Steam, KH1's entry builder hooked): on the window's first frame each KH2 shape is measured into KH1's grip bone (280) axes (`bodyhits::grip_local`); after that KH1's pass places it from KH1's grip each pass (`bodyhits::grip_guard_keep`, result word held at clashed, as t861), and KH2's rig isn't posed for the window. Log line "measured onto KH1's grip" shows the offsets. Assumption: the shape keeps its place on the blade through the window.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-roll-guard-native.md §11.

### 2026-10-29: Stage 910 (testing branch: the spell scripts' lock-on straight from KH1)
- The owner (chat: "do all 3 and finish whatever is left", item 3).
  - Before: `takeover::native_spells` worked out a lock id from our hit state's targets (`kh1_lock` over `Strikes::look`) for the spell scripts' world; the world never read it (its lock-on is KH1's own, `reflect::KhWorld::locked`, `takeover::kh1_locked`).
  - Now: the lock passed is KH1's own (`kh1_locked`); `kh1_lock` removed. Our hit state's look stays for the spells' own fallbacks and Reflect's barrier.
  - `cargo test` not run for `kh1-hook` here (no crate registry reachable).

### 2026-10-29: Stage 909 (testing branch: KH2's body guards placed by KH1's own collision pass)
- The owner (same request, item 1).
  - Before: KH2's guard windows on his body (Aerial Dodge, Aerial Recovery, Explosion, Magnet Burst) were KH1 guard objects we moved each frame onto KH2's shapes on KH2's rig, posed by us.
  - Now (Steam, KH1's entry builder hooked): each shape is a KH1 guard object (clash type 4, priority 100, the ATKP's arc) that KH1's pass places from KH1's state each pass (`bodyhits::body_guard_keep` / `body_guard_entries`, the body hits' placement: his root and facing, KH1's bone 70 for KH2's 67, KH2's sweep); KH2's rig isn't posed for them, nor for KH1's own blade (`combat::guard_needs_pose`). A window with a shape KH1 can't place keeps the old way. With Guard Shape KH1, one KH1 blade guard a window (was one a shape, the same object several times).
  - Left: Guard Shape KH2's Keyblade shapes stay on KH2's posed rig (KH2's weapon bones against KH1's grip not checked).
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-roll-guard-native.md §10.

### 2026-10-29: Stage 908 (testing branch: t907's build errors, and unused imports)
- The owner's build of t907: "cannot find value `kh2_motion` in module `super::kh2slots`" (`pose.rs`, `shared.rs`).
  - Cause: t907 removed `kh2slots::kh2_motion` as unused; `pose` and `shared` pass it as a function (no call parentheses), which the search for its calls missed.
  - Now: `kh2slots::kh2_motion` back. The build's unused-import warnings cleared: `form` (`shots_step` re-export), `formmagic` (`CastMotions`, `Character`), `hurt` (`Hit`, `StateName`, `AtomicU64`), `shared` (`StateName`, `AtomicU8`), `reflect` (`Character`), `takeover` (`Button`, `Env`, `EventKind`, `Input`). `chest`'s deprecated `fetch_update` left as it is (its replacement needs a newer Rust than the workspace's 1.75).
  - `cargo test` not run for `kh1-hook` here (no crate registry reachable).

### 2026-10-29: Stage 907 (testing branch: dead code of the deleted controller, and two warnings)
- The owner (chat: "okay do the small leftovers go ahead").
  - The trace's KH2 State category removed (its columns, `trace::Kh2`, `Row::kh2`, the ini's `trace_kh2_state`, the debug menu's "KH2 Controller" group, now "Drive Form" with Form alone): it held the deleted controller's state and stayed empty since t901. Its bit stays unused. The CSV loses those 11 columns.
  - `formexp::generation` and its counter removed (the deleted `form::abilities` read it; a form level-up reaches the form's abilities through `combomods::abilities_now` each frame); `kh2slots::kh2_motion` removed (only `fapanel`'s unused `ours` read it); `shared::control`'s unread `borrow = NONE` removed.
  - `cargo test` not run for `kh1-hook` here (no crate registry reachable).

### 2026-10-29: Stage 906 (testing branch: KH2's repel and Counterguard on KH1's own guard)
- The owner (same go-ahead; the item "KH2's repel and Counterguard on Guard Animation KH1").
  - Before (t903): with Guard Animation KH1, KH1's own guard blocked by KH1's own meeting and nothing of KH2's followed.
  - Now: `combomods::kh1_guard_frame` (each frame Sora is posed, `kh2slots::frame`): while KH1's guard plays (request 0xD4), its guard object (`combat::kh1_guard_object`: Sora's live attack object on row 0x3D's data, Checked (files) and (decomp)) is known to KH1's meeting as ours (`combat::kh1_guard_met`), and an enemy attack it meets (KH1's two directional tests passed) is KH2's guard reaction with the Recoil row at KH2 (`guard_repel`, refact self 1 as KH2's guard ATKP 161: the repel 42 on its request of ours, pushed back, KH2's 80 frames without damage, Counterguard in it). With Recoil at KH1, KH1's own block as it is. Cost: two reads a frame, a 64-slot pool look only while KH1's guard plays.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-roll-guard-native.md §9.

### 2026-10-29: Stage 905 (testing branch: the trace's Config columns for the two guard rows)
- The owner's build of t904: "error[E0080]: evaluation panicked: assertion failed: fields(COLUMNS[1].1) == CONFIG" (`trace.rs`).
  - Cause: t903 / t904 added two move styles (`styles::Kind::GuardAnim`, `GuardShape`) without their names in the trace's Config header, which the build checks against `Kind::EVERY` (t-config-columns).
  - Now: `cfg_guard_anim`, `cfg_guard_shape` at the header's end, in `Kind::EVERY`'s order (29 Config columns, inside the row's 32 bits).
  - `cargo test` not run for `kh1-hook` here (no crate registry reachable).

### 2026-10-29: Stage 904 (testing branch: Guard Shape row: KH2's Keyblade guard on KH1's own blade)
- The owner (chat: "for guard I want you to make a toggle in combat to change the guard animation (a)", then "Both, two rows"; the guard-window question's answer (a)).
  - New Config row, Combat > Guard Shape (`styles::Kind::GuardShape`, ini `guard_shape`, KH1 by default as every row): KH1, each of KH2's Keyblade guard windows (the guard press 173, its repel 42, Guard Break) is a KH1 guard object made by KH1's row maker from KH1's own guard row's data (Sora's row 0x3D, data 0x4DA8: bone 0x30, so KH1's own builder places it along KH1's Keyblade each frame; clash type 2; Checked (files) and (decomp)), KH2's window's ATKP arc on it, always live while KH2's window is open (`combat::Strikes::blade_guard`): nothing of it placed by us. KH2, KH2's guard shapes placed each frame on KH2's rig as before (t861). The repel (KH2's guard reaction, `guard_repel`) follows KH1's meeting either way (`keyblade_guard_meeting`). Body guards (weapon 0) unchanged.
  - **Assumption**: KH1's builder places a guard object of bone 0x30 on the blade as it does KH1's own guard's (KH1's guard row uses that data).
  - Not checked in game; `cargo test` not run for `kh1-hook` here (no crate registry reachable). docs/kh1-roll-guard-native.md §9.

### 2026-10-29: Stage 903 (testing branch: Guard Animation row: KH1's own guard on the guard press)
- The owner (same request).
  - New Config row, Combat > Guard Animation (`styles::Kind::GuardAnim`, ini `guard_anim`, KH1 by default as every row): KH1, the guard press (Square standing, Dodge Roll KH2, KH2's Guard equipped: `combomods::guard_press`) starts KH1's own guard as KH1's Square handler does (`0x1402A7B30`'s guard branch, Checked (decomp)): turned toward KH1's target (`0x1402A3B00`), KH1's guard row 0x3D (`0x1402A3980`: request 0xD4, KH1's guard animation and KH1's guard object), the idle queued, KH1's count 1 (`combomods::kh1_guard`; first bytes checked, Checked (exe)). KH2, KH2's guard press (motion 173 on our request) as before. With KH1's guard, KH1 decides the block (KH1's own meeting); KH2's repel and Counterguard come with KH2's guard only.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-roll-guard-native.md §9.

### 2026-10-29: Stage 902 (testing branch: the spell scripts' enemies from KH1's own enemy walk)
- The owner (same go-ahead).
  - Before: the KH2 spell scripts' world (`reflect::world_now`, used by `formmagic` and Reflect) listed the enemies from our hit state's look (`Strikes::target_actors`: the hittable ones within reach of him with body parts).
  - Now: KH1's own enemy walk (`combomods::kh1_foes`: `0x140291AB0`, live, another team, not hidden, Checked (decomp); first bytes checked, Checked (exe)); our hit state's list only where that call isn't the expected code. Far enemies are in the scripts' world too now, as in KH2's (the scripts measure their own ranges).
  - **Assumption**: no KH2 spell script expects only nearby enemies in its world.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-drive-forms-native.md §7.4.2.

### 2026-10-29: Stage 901 (testing branch: our KH2 controller object deleted)
- The owner (chat: "go ahead with whats still open").
  - Before: `takeover`'s driver (a `Character` copy of KH2's controller with its rig, input state, last frame, hit state, menu work and form state) still existed for the hit sweeps, the spell scripts' world and the trace.
  - Now: deleted. The hit state (`combat::Strikes`: KH1's enemies' bodies, guards and shots as our hit records see them, Reflect's barriers, the spells' attack objects) is its own (`takeover::STRIKES`); `kh2_hits` poses KH2's rig from base Sora's moveset (`kh2.character`, only read); `Strikes::strike_as` / `guard_step` take no controller (`Option<&mut Character>`, `None` on KH1's states: the controller's own wall / hit counters and clash had no effect there; KH2's reaction numbers from KH2's data). The trace's KH2 columns stay empty (no KH2 frames). `release` logs as before without the driver's counters.
  - Not checked in game; `cargo test` not run for `kh1-hook` here (no crate registry reachable). docs/kh1-drive-forms-native.md §7.4.2.

### 2026-10-29: Stage 900 (testing branch: the ability menu's own work on its own)
- The owner (same go-ahead).
  - Before: `menu::Growth` (KH2's abilities given in KH1's lists, their boxes and menu names) lived in our KH2 controller object and wrote Sora's abilities into its `Character`.
  - Now: `menu::growth_frame` runs it from a static of its own (`GROWTH`), each frame of Sora's on KH1's states; `Growth::update` writes nothing into a controller (what reads abilities reads KH1's lists itself, `kh1_abilities_now`, t893).
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-drive-forms-native.md §7.4.2.

### 2026-10-29: Stage 899 (testing branch: the Drive Form's state on its own)
- The owner (same go-ahead).
  - Before: the Drive Form's state on KH1's states (`form::Drive`: the change, the form, its MP drain) lived in our KH2 controller object, and the arrival / revert swapped that object's `Character` for the form's (only read by what is native now) and kept Sora's and the form's abilities in it (`form::abilities`).
  - Now: `form::DRIVE`, its own; `native_frame` takes no controller; the arrival takes Sora's abilities from KH1's lists (`combomods::abilities_now`), the form's by KH2's rule as `abilities_now` gives them in the form; `form::battle` by the form now (`native_form`). The old controller's `swap` / `revert` / `abilities` / `native_data` removed.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-drive-forms-native.md §7.4.2.

### 2026-10-29: Stage 898 (testing branch: KH2's movement numbers from the moveset's records, written only when they change)
- The owner (chat: "go ahead", item 2 of what was left: the movement numbers off our KH2 controller object).
  - Before: `takeover::refresh_kh2_numbers` read the walk, run, jump, gravity, air and ground inertia numbers and High Jump's height from our KH2 controller object every frame and wrote them for KH2's physics on KH1's states.
  - Now: `takeover::kh2_numbers`: the moveset's own (`kh2table`: its `prty` numbers and `fmab` levels, built once; base Sora's or a Drive Form's on KH1's states), High Jump at his level from KH1's lists (`combomods::abilities_now`, read at most every 30 game frames), written only when the moveset or the level changes (`shared::physics::set_numbers` now says whether they went in). The ability menu's own work (`menu::Growth::update`: abilities given, boxes, names) still runs each frame as before.
  - Not checked in game; `cargo test` not run for `kh1-hook` here (no crate registry reachable). docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 897 (testing branch: the growth abilities' levels and the remaining ability checks from KH1's lists)
- The owner (same go-ahead, item 1).
  - Before: Aerial Dodge, Dodge Roll, Quick Run and KH2 Glide took their level and `fmab` record from our KH2 controller object's copy (`takeover::kh2_air_dodge*`, `kh2_dodge_roll_growth`, `kh2_quick_run_growth`, `kh2_glide_growth`, `kh2_glide_open`, `kh2_square`), as did the guard's all-round check (`hurt`), the Quick Run-for-roll check (`shared`) and the debug menu's growth page.
  - Now: the level from KH1's lists now (`combomods::abilities_now`, t893), the record from the moveset's (`kh2table::Table::growth_level`, Sora's or a Drive Form's `fmab`). `takeover::with_abilities` removed: nothing reads the controller object's ability copy. Comments in `form` corrected (Wisdom runs on KH1's states; Final is refused, t873).
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 896 (testing branch: KH2's combo length through the one hook on KH1's stats builder)
- The owner's t891 log: "hook party stats at 0x...5130: unexpected bytes [e9, ...], not hooked".
  - Cause: t891 hooked KH1's stats builder (Steam `0x1402A5130`) a second time from `kh1picker`, installed before `menu`'s own party stats hook on the same function (Epic RVA `0x2A2FA0`), which then found our jump in its first bytes and refused: since t891, Valor's second Keyblade's Strength / MP on Sora's stats and the debug menu's stat locks were off.
  - Now: no hook of `kh1picker`'s; `menu`'s party stats hook calls `kh1picker::after_stats` after KH1's own builder (KH2's length into Sora's `+0xD4` / `+0xD5` where it is his, as t891 did), Steam only.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 895 (testing branch: Auto Aim on KH2's primary target from KH1, and KH2's turn windows on KH1's target)
- The owner (chat, after t891: "go ahead with whats open don't need my input just finish it").
  - Before: since t889 KH2's walk took KH1's own target for the press whatever the Auto Aim setting (it only changed the hop's controller ask), and KH2's turn windows (`kh2slots::turn_to_lock`) turned him toward the controller object's primary target (`takeover::kh2_primary_target`, its own target list).
  - Now: `combomods::kh1_press_target`: Auto Aim on, KH1's own target for the press (combat info `+0x54`, Checked (decomp)); off, KH1's lock-on alone (KH2's rule without its auto target). The turn windows turn him toward the same target (`combomods::kh1_primary_point`). `takeover::kh2_primary_target` removed.
  - Not checked in game; `cargo test` not run for `kh1-hook` here (no crate registry reachable). docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 894 (testing branch: the Square moves, Counterguard, Retaliating Slash and Valor's row 11 on KH1's own state)
- The owner (same go-ahead).
  - Before: these asked our KH2 controller object (`takeover::kh2_would_take_square`, `kh2_would_take_row`, `kh2_retaliating_slash`).
  - Now: `character_core::HostPress::square` (the exe's order 0x25, 0x0C, 0x26 and checks, Steam KH2 `0x1403A81F0` / `0x1403A9950`, Checked (decomp)) and `HostPress::forced` (a state's forced row: a new combo's press, its conditions), on KH1's state (`combomods::Kh1Press::square` / `forced` / `retaliating_slash`), with their hops (t892); Combo Master counts as connected, as KH2's. The three `takeover` asks removed: no combo press asks the controller object now. `crates/character-core/tests/host_combo.rs` checks both against the controller's own on 4000 made-up cases (run here: passes).
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 893 (testing branch: Sora's KH2 abilities read from KH1's own lists at the moment of use)
- The owner (same go-ahead).
  - Before: the combo's checks read the copy of his abilities our KH2 controller object refreshes once a second (`takeover::with_abilities`).
  - Now: `menu::kh1_abilities_now` reads his ability list (save `+0x44`), the shared list (`+0x599`) and the KH2 list as KH1 holds them now, through our table of which KH2 ability each entry is (`growth::Kh1Sora::kh2`; KH1 has no KH2 numbers), with the debug menu's growth overrides and the `abilities` setting's test sets; in a Drive Form on KH1's states, the form's by KH2's rule (`form::form_abilities`, now `pub`). `combomods::abilities_now` (once a game frame at most) is used by the walk, the length (`kh1picker::sync_length`), Finishing Plus (and `kh2slots`' Finishing Plus window), the guard, Counterguard and Aerial Recovery's checks. Other systems (glide, Quick Run, the dodge roll) keep the controller's copy.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 892 (testing branch: a combo row's hop worked out on KH1's own state)
- The owner (same go-ahead).
  - Before: the hop of the row KH2's walk took on KH1's state came from our KH2 controller object (`takeover::kh2_hop`: `Character::would_take_row` with its own target list and speed).
  - Now: `character_core::HostPress::hop` (Steam KH2 `0x1403D78D0`, Checked (decomp)): the height and sideways speed for KH1's target for the press, from his own target shape's centre (the moveset's `target_lift` / `target_radius`, now in `kh2table`), with no target the row's own along his facing (the stick on a combo's first press, as KH1's pad and camera give it); a moving row (an allow-movement range: Wisdom's air shots) keeps his speed in KH1's own fields (`+0x3C0` in the air, `+0x20` x `+0x2C` on the ground). Checked against the controller's own on 4000 made-up cases (run here: passes). `takeover::kh2_hop` removed.
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 891 (testing branch: KH2's combo length kept in KH1's own stats)
- The owner (chat, the native plan for "which attack a press gives, and the Finishing Plus counter", part 4, answer 2 (a): "go ahead").
  - Before: at the combo style KH2, `combomods::kh2_length_for_picker` wrote KH2's length into Sora's stats length byte (`+0xD4` ground, `+0xD5` air) for the call of KH1's picker and put KH1's back after it, so every other reader saw KH1's length.
  - Now: `kh1picker::sync_length` keeps KH2's length (KH2's base, Combo Plus / Air Combo Plus, Negative Combo, `kh2table::Table::length`) in the byte while it is his (base Sora with the Base / Aerial Base Combo at KH2, or a Drive Form on KH1's states). Written after KH1's own stats builder `0x1402A5130` runs (hooked, prologue `48 89 5C 24 10 48 89 6C 24 18`, Checked (exe)) and at each Attack press in KH1's pickers; KH1's own length (copies of ability 6 / 7 + 3, counted by `0x140284BE0`, Checked (decomp) and (exe)) goes back when the style is KH1 again. KH2's walk reads it from there (`Kh1Press::read`). `kh2_length_for_picker` removed.
  - Side effect: the cast step (`formmagic`, KH2 magic) reads KH2's length at the combo style KH2 now, where it read KH1's.
  - Not checked in game; `cargo test` not run for `kh1-hook` here (no crate registry reachable). docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 890 (testing branch: the Finishing Plus count and the combo's last step in KH1's combat info)
- The owner (chat, the same plan, part 3: "go ahead").
  - Before: KH2's finisher count `+0xD75` and the combo's last step were statics of ours (`combomods::FINISHERS`, `SPENT`, `LAST`).
  - Now: in Sora's KH1 combat info (his `+0x14C` record, KH1's nine 0x60-byte records at `0x14296B1E0`, Checked (decomp)): `+0x44` KH2's finisher count (1 at a new combo, one more for each press taken in a Finishing Plus window, a finisher's or Magnet's mid-combo, up to 100: Steam KH2 `0x1403D6CA0`, Checked (decomp)); `+0x45` the last step's flags, `+0x46` its KH1 request, `+0x5C` KH1's count at it. A record handed to Sora anew is cleared first (`combo_info`). A press while the combo's finisher still plays keeps the count even when KH1's count went back to 1. The Square moves' carried combo counts a mid-combo Magnet spend too, as KH2's count does.
  - **Assumption**: KH1 reads and writes none of those bytes through another path than the actor's `+0x14C` handle (none found through it, Checked (decomp)).
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 889 (testing branch: KH2's row walk inside KH1's pickers on KH1's own state and targets)
- The owner (chat, the same plan, part 2, answer 1 (a): KH1's own targets).
  - Before: KH1's pickers asked our KH2 controller object (`takeover::kh2_walk`, `kh2_rows`, `kh2_ability_row`) which row a press gives, with its own copy of his abilities, target list, facing and lock set from KH1 at each press.
  - Now: `combomods::Kh1Press` reads KH1 at the press: its count, the press's target in the combat info `+0x54` (written by KH1's Attack step `0x1402A7510` before its picker, Checked (decomp)) as KH2's primary target, every enemy's lock points by KH1's own calls (`0x140291AB0`, `0x1402B0110`, `0x1402B04C0`, as KH1's `0x1402A7320` makes them, Checked (decomp); first bytes checked, Checked (exe)), his place, facing and the KH2 motion playing; KH2's walk on it (`character_core::HostPress`). Used by the KH2 style's press and Finishing Plus's next finisher (`kh2_press`), the KH1 style's ability rows (`picker`) and both finisher walks. `kh2_walk`, `kh2_rows`, `kh2_ability_row`, `kh2_would_take_launch` and the drop-ahead flag removed from `takeover`.
  - **Not native yet**: the hop of the row taken (`takeover::kh2_hop`: the controller's `would_take_row`, KH1's press target as its lock); his abilities as KH2's numbers from our reading of KH1's lists.
  - **Assumption**: KH1's lock point radius stands for the target's height too (as before); each lock point of each enemy is a target now (before: one a body near him).
  - Not checked in game; `cargo test` not run for `kh1-hook` here. docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 888 (testing branch: KH2's combo row records for KH1's pickers, and KH2's walk as a function of a host's state)
- The owner (chat, the same plan, part 1: "go ahead").
  - `kh2table`: one table a moveset (base Sora's, each Drive Form's), built once from KH2's data as loaded (PTYA rows in KH2's order, the KH1 row of ours that plays each, the motions KH2 has clips of, the combo rules and jump height). `combomods::ours_for` finds a row's KH1 row through it first.
  - `character_core::HostPress`: KH2's Attack press (`0x1403D6CA0`'s steps, the walk `0x140402D20`), the KH1 style's ability rows, row range checks and normal finisher, from inputs a host gives alone; `crates/character-core/tests/host_combo.rs` checks it against the controller's own press on 4000 made-up cases (run here: passes). Nothing plays differently with this stage alone.
  - docs/kh1-anim-slots.md §7.z.11.

### 2026-10-29: Stage 887 (testing branch: KH2's spells only on KH1's script machine: no fallback onto our own when the hook is missing)
- The owner (chat, after t886's question: "continue with (b) spells run native now").
  - Before: with KH1's interpreter hook not in (a hook or first-byte check failing at start, a non-Steam build), `Magic::native` was none and every KH2 script (Fire, Blizzard, Thunder, Cure, Reflect, Wisdom's shots) ran on our own machine, stepped by `Magic::update`.
  - Now: `formmagic` and `reflect` start a cast or Wisdom's shot script only with KH1's machine as the host; without it the start is an error (`kh2script::off`), logged "didn't start" as any failed start. `kh2script::install` logs that KH2's spells won't cast when it can't hook. Our machine stays only for a host-less `Magic` (`kh2-magic`'s offline tests).
  - Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-magic-native.md §8c.

### 2026-10-29: Stage 886 (testing branch: KH2's spell scripts with no fallback onto our own script machine when KH1's pools are full)
- The owner (chat: "We don't need a fallback for kh1's pools so this should be removed", about the spell scripts' own machine as a fallback when KH1's pools are full).
  - Before: with the KH1 interpreter hook in, a cast (or Wisdom's shot script) KH1 had no free record, work slot or spell task entry for ran on our own machine (`kh2_bdx::Vm` on a private arena, stepped by `Magic::update`); a thread (`trap_magic_start_thread`) ran inside its parent's step on our side while 8 or fewer of KH1's 64 records were free (`RESERVE`), or when KH1's child start didn't give a child; a cast the host couldn't take (`started` false) ran on our side too.
  - Now: with a host set (`Magic::native`) there is no fallback. `machine` giving none, or `started` false, is an error from `Magic::cast` / `Magic::object_effect` (logged by `formmagic` / `reflect` as "didn't start"; a process KH1 made for it ends at KH1's next step, as before). A thread the host can't make doesn't start and the call gives a null record (`kh2-magic` host trap 2,20). `kh2script`: `RESERVE` removed (a thread takes any free record); the free-record / work-slot / task check stays only so KH1's allocators, which check nothing (`0x1402BD5B0` returns nothing with none free, `0x1402BD420` takes the slot past its 32, Checked (decomp)), are never called full. Our machine stays only for a host-less `Magic` (the offline tests, and a hook or first-byte check failing at start).
  - **Assumption:** KH2's `trap_magic_start_thread` gives 0 when it can't start a thread (not checked in KH2's decomp); no spell script reads the record back as far as `kh2-magic`'s calls go.
  - Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-magic-native.md §8c.

### 2026-10-29: Stage 885 (testing branch: Air Physics KH2's hover on KH1's own air cast animation as KH1 motion data)
- The owner (chat, after t882: "go ahead with what you need"; §14.14 of docs/kh1-anim-baking.md).
  - Before: `shared::physics::kh1_cast_hold` (Air Physics KH2, KH1's own air cast animation; the tester's "hold him like kh2") counted 60 Hz frames itself from the first frame it saw the air cast request and searched KH2's air cast clip for the request (`kh2slots::CASTS`) for its no-gravity range every frame.
  - Now: `kh2slots::kh1_copies` also copies KH1's air cast slots (79 Fire, 80 Blizzard / Gravity, 81 Thunder / Cure / Stop / Aero, 82 Fire's second part; the file's request table) with each KH2 air cast's hold at its own frames (`no_gravity_spans`, none for a cast that allows movement all through) as our level event 8.(0x100 + the KH2 cast) (`kh1_data::slot::RANGE_KH1_AIR_CAST_HOVER`): one track per KH2 cast, as one KH1 slot plays for several spells whose KH2 casts hold differently (81: Thunder 64, Cure 67). `kh1_cast_hold` reads it with KH1's own reader at KH1's time in the cast (`kh1_copy_level`); our own count and KH2's clip only where no copy plays. `slotcache::BAKE_FORMAT` 13.
  - One change in timing: KH1's time in the cast, not our count from the request. Through a blend into the cast KH1 holds its time at the start (`0x14029F0A0`, Checked (decomp)), as KH2 reads its own ranges at a blending motion's start (`0x1403C6740`, Checked (decomp)); our count went on through it. So after a blend the hold starts and ends that blend's length later than before, as KH2's hold would on its own cast. With no blend: the same frames.
  - Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.16.

### 2026-10-29: Stage 884 (testing branch: Quick Run's stop ends on KH1's own motion length)
- The owner (chat, after t882: "go ahead with what you need"; §14.14 of docs/kh1-anim-baking.md).
  - Before: `quickrun`'s step ended Quick Run's stop (our slot of KH2's 204) when KH1's time reached base Sora's 204 clip length, looked up in KH2's moveset every frame of the stop, in Wisdom too.
  - Now: the length KH1 keeps for the motion playing (`+0x280`; KH1's motion start sets it from the motion's header length `0x1403948C0`, Checked (decomp)), which for our slot of 204 is its clip's length. Base Sora: the same frame as before. Wisdom (its own 204 on our Quick Run rows): its own 204's length now, where base Sora's was used; KH2 ends the stop on the motion it plays. No hook.
  - Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.16.

### 2026-10-29: Stage 883 (testing branch: the ATKP of KH2's hit window at a hit as KH1 motion data: our level event 8.16)
- The owner (chat, after t882: "go ahead with what you need everything is working"; the first of the runtime KH2-clip reads left after item 5, docs/kh1-anim-baking.md §14.14).
  - Before: at each hit of one of our rows' attack objects (`kh1rows::kh2_atkp`, which `recoil` and KH2's damage read) and of a native swing on KH1's rows (`native_atkp`, `kh2damage`), the KH2 motion's clip was searched for the hit window at KH1's time (`window_atkp`: the right Keyblade's, else his body's; the move's first at all where none).
  - Now: `kh2slots::kh1_tracks` bakes the ATKP at each frame as our level event 8.16 (`kh1_data::slot::RANGE_ATKP`): a key wherever the frame's ATKP changes (`a` 1 and the ATKP in `c` inside a window, `a` 0 and `NO_ATKP` outside), every key's `d` the move's first ATKP, a first key at 0; frame for frame `window_atkp`'s rule at `t as u32` (`clip_atkp`). On Steam both readers take it from KH1's own reader (`kh2slots::atkp_now`); KH2's clip is read only where that can't be. `slotcache::BAKE_FORMAT` 12.
  - Same ATKP at every frame as before (the bake runs the old rule on every frame of the clip). The second Keyblade's windows stay its own object's (`bodyhits::atkp`), as before. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.16.

### 2026-10-29: Stage 882 (testing branch: KH2's windows on KH1's own motions as KH1 motion data: copies of KH1's motions as slots of ours, our level events 8.10 .. 8.15)
- The owner (chat, after t881 tested: "everything is working just do all the parts you need to do (a)"; what-is-still-missing item 5 part 7, option (a)).
  - Before: four windows were placed on KH1's own motions (or, for one, on our slot from another KH2 motion) by share every frame, from KH2's clips: KH2 Fire's combo window on KH1's casts at Magic Combo KH2 (`formmagic::kh1_cast_window`), KH2 Magnet's Finishing Plus window on KH1's Stop as Magnet (`magnet_window`), KH2 Thunder 63's window on Gravity / Stop / Aero as the combo's finisher (`finisher_spell_window`, on our cast slots), and KH2 155's / 183's window on KH1's own finishers with Finishing Plus (`combomods::kh1_finisher_window`).
  - Now: `kh2slots::kh1_copies` copies the KH1 motions those requests play (the file's request table; one copy per KH1 slot, plain entries only) as slots of ours: KH1's own motion unchanged (channels, events, length), plus our level events at the same share of its length, built once into the slot cache. 8.10 Fire 56's combo window on Fire 0x36 / 0x3D, Blizzard 0x37, Thunder 0x38, Cure 0x39, Gravity 0x3A, Aero 0x3C; 8.11 Magnet 68's window (Sora's) and 8.12 (Wisdom's 68, its own window and length) on Stop 0x3B; 8.14 / 8.15 155's / 183's window on KH1's combo requests 0xC8..0xDB. `make` bakes 8.13 (Thunder 63's window by share) onto our ground casts of Gravity, Stop and Aero. `place` puts the copies after our moves (`put_motion`, shared with the moves now) and `extend` adds them to each set where the set's entry holds that KH1 motion (by signature); `redirect` points a request at its copy wherever its entry is KH1's own (never where one of our KH2 slots is on it), every frame as before. To KH1's slot-by-motion lookups a copy is the KH1 slot it copies (`MOTION_AT`), so KH1 and our code (`pose::sora_slot`, the swing slots 98..104) see KH1's own motion. The four readers use `kh2slots::kh1_copy_level` (copies) and `kh1_level` (8.13 on our casts), KH1's own reader, and keep the share reading only where no copy plays.
  - KH1 (Checked (decomp)): the length `+0x280` the share readings used is the motion's header length (`0x14029F0A0`'s start sets it from `0x1403948C0`, the motion's `+4`), so the baked windows are on and off at the same times. KH1 never asks for type 8 (t868), so a copy plays exactly as KH1's own.
  - **Assumption**: KH1's ability finishers' requests are within 0xC8..0xDB (its row table is loaded at run time, so the bake can't read which rows are finishers). A KH1 finisher with no copy is logged ("has no copy of ours") and keeps the share reading. Nothing else changed: the conditions (Magic Combo KH2, the roles, Finishing Plus, a finisher left) are still checked when read.
  - `slotcache::BAKE_FORMAT` 11. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.15.

### 2026-10-29: Stage 881 (testing branch: KH2's combo window and free state as KH1 motion data for the Square moves: our level events 8.8 / 8.9)
- The owner (chat, after t878: "just do the rest of the parts"; what-is-still-missing item 5 part 9).
  - Now: `kh2slots::kh1_tracks` bakes KH2's combo window (range id 11) as our level event 8.8 (`kh1_data::slot::RANGE_COMBO`) and its free state (range ids 0 / 1 / 2) as 8.9 (`RANGE_FREE`) on every slot. `kh2slots::windows_now` gives (motion, frame, combo, Finishing Plus, free) from 8.8 / 8.6 / 8.9 with KH1's own reader on Steam (KH2's clip where that can't be read), and KH2's Square handler's checks use it: base Sora's Square moves in a swing (`combomods`, Finishing Leap / Upper Slash / Horizontal Slash) and Valor's Square (`combomods::valor_square`). `kh2slots::cast_cut_open` (a jump or Square out of a cast) reads 8.9 the same way. `slotcache::BAKE_FORMAT` 10.
  - Why our own 8.8 / 8.9 and not KH1's 5.0 / 0.0: on cast slots KH1's 5.0 is the cast's whole combo window (ranges 11 and 35, `cast_follow`), and 0.0 is the template's on slots built with `Leave::Template`; KH2's Square handler reads range 11 and the free state exactly (Checked (decomp) earlier, `0x1403A9950` / `0x1403BFB30`, as `combomods::valor_square` notes).
  - The plan's other half of part 9, the reaction command's first frame, was read only by the old controller's step (`takeover`), deleted at t874; nothing reads it now (Checked (code)).
  - Same frames as before. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.13.

### 2026-10-29: Stage 880 (testing branch: KH2's Keyblade re-arm as KH1 motion events: our edge key KH2_REARM)
- The owner (chat, after t878: "just do the rest of the parts"; what-is-still-missing item 5 part 8).
  - KH2 (Checked (decomp), as at t-rehit): a new attack starts when the open attack range's ATKP or group differs from the last frame's, or a range opens after a frame with none.
  - Now: `kh2slots::rearm_keys` bakes each new right-Keyblade attack after the motion's first as a type-6 edge key of ours (`kh1_data::slot::KH2_REARM`, `b` the KH2 motion, `c` the ATKP), at KH2's range start time (`ks::kh2_trigger_time`), added before the frame triggers so the edge budget never drops it. On Steam KH1's motion pass fires it and our wrapped sound handler (`sound_event`) re-arms the attack objects of ours (`kh1rows::rearm_rows`, unchanged); `keyblade_rearm` (the per-frame read in `takeover::physics`) now returns at once on Steam and stays for other builds. `slotcache::BAKE_FORMAT` 9.
  - Timing (Checked (decomp)): KH1's physics step `0x140298230` (where `keyblade_rearm` ran) queues the actor, and the motion pass `0x140298730` → `0x14029F0A0` runs after it, so the re-arm now comes in the same pass that moves the time into the new attack, not at the next physics step: never later than before. A step that jumps a frame (30 fps) no longer skips a one-frame boundary. **Assumption**: calling `rearm_rows` from inside KH1's motion pass is safe (it runs KH1's own re-arm on our attack objects only, as it did from the physics step).
  - Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.12.

### 2026-10-29: Stage 879 (testing branch: KH2's Finishing Plus window and a cast's combo window as KH1 motion data: our level events 8.6 / 8.7)
- The owner (chat, after t878: "just do the rest of the parts, everything seems to be working"; what-is-still-missing item 5 part 6).
  - Now: `kh2slots::kh1_tracks` bakes KH2's Finishing Plus window (range id 50) as our level event 8.6 (`kh1_data::slot::RANGE_FINISHER`) on every slot, and `make` bakes a cast's KH2 combo window (`combo_kind`: range id 11, or 35 naming the cast's own motion) as 8.7 (`RANGE_CAST_COMBO`) on every cast slot, finisher casts too. On Steam `finisher_window` (KH1's combo bit on our finishers with Finishing Plus), `cast_finisher_open`, `cast_attack_window` and `cast_magic_window` read them with KH1's own reader (`kh1_level`) instead of KH2's clip; KH2's clip is still read where KH1's reader can't be (off Steam, or the reader's first bytes not as expected). Their conditions (Finishing Plus equipped, a finisher left, base Sora on KH1's states) are checked at the read as before: they change at runtime, so they can't be in the data. `slotcache::BAKE_FORMAT` 8.
  - Counterguard (`combomods::counter_window` / `counter_open`): no range to move. KH2 takes Counterguard on any frame of its guard repel (motion 42), and that is already read from the slot KH1 plays (Checked (code)).
  - Not changed: KH1's own finishers' and casts' windows placed by share (`combomods::kh1_finisher_window`, `formmagic::kh1_cast_window`, `finisher_spell_window`, `magnet_window`): those run on KH1's own motions, not our slots (part 7). Same frames as before. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.11.

### 2026-10-29: Stage 878 (testing branch: KH2's turn toward the target as KH1 motion data: our level event 8.5)
- The owner (chat, after t877: "continue with part 4 if you haven't done it already"; part 4 was t872, so this is what-is-still-missing item 5 part 5).
  - KH2 (Checked (decomp), as at t-turn): range 28's handler (`0x14040D810` → `0x1403D4B10`) turns him toward his primary target by at most twice the range's parameter (radians) a motion update, while start <= t < end. Sora's and Wisdom's motions change the rate between ranges of one motion (Checked (files): `P_EX100_MAGF.mset` 360 / 361 / 364 ... 22 degrees then 4; `P_EX100.mset` 304, 312, 318); no two turn ranges overlap in either file.
  - KH1 (Checked (decomp)): its level reader `0x140394770` returns the address of the last key at or before the time in the motion's own key list (time, `a`, `b`, `c`, `d`), 0 before the first; so a key can carry a value.
  - Now: `kh2slots::kh1_tracks` bakes range 28 as our level event 8.5 (`kh1_data::slot::RANGE_TURN_TO_LOCK`), one "on" key per range with its rate as an f32 in `c` (`EventTrack::valued`, with a unit test), "off" at its end unless the next range starts there. On Steam `kh2slots::turn_to_lock` takes the rate from KH1's reader (`kh2slots::kh1_key`, which `kh1_level` now uses) instead of searching KH2's clip; off Steam the clip as before. `slotcache::BAKE_FORMAT` 7 (the slots rebake once).
  - Not changed: the turn itself (target pick, the 50 distance, KH1's facing fields, the move vector, the travel turned with him) is the same code, still run once a frame before KH1's physics frame: KH1 has no turn-toward-target of its own to hand it to. Same frames and rates as before. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.10.

### 2026-10-29: Stage 877 (testing branch: the old KH2 controller's checks left in other modules deleted)
- The owner (chat, after t874: "if there is anything left to deprecate?", then "just do all of them and report back when your done").
  - Folded to their KH1-states answer (KH2's controller never has Sora since t874, so `kh2_has` was always false and `driver_form` / `with_kh2` only served it): `bodyhits`, `cmdmenu`, `combomods`, `form` (`accept_now`, `usable`, `choose`, `native_enabled`), `formmagic::kh1_spell_start`, `reflect` (`usable`, `greyed_why`, the airborne branch), `hitreact`. `hurt::hurt` keeps only its KH1-states path (its controller branch, `Fate`, the re-hit grace and its statics gone). `pose::wanted` mode 0: the scripted check, then KH1's own pose (`follow`); the controller's pose route is gone.
  - The ini's `shared` setting (field, parse line, help lines), `shared::controller_cut_off` and `setting_on` deleted; `shared::on` is always true. `shared::note_cut_off` now says "refused (only the old KH2 controller ran it; it is deleted)".
  - KH1's control step: a fresh ground jump before KH1's states have him is let through without asking whether the controller's move can be left (`kh2_jump_open`, which read the controller's character, deleted). Its strip of the jump / Square bits during one of KH1's commands is kept (still reachable: the input guard after a command).
  - The controller's per-step hooks nothing called any more, deleted: `form::before_step` / `after_step` / `arrive` / `takes`, `formmagic::before_step` / `waiting` / `affordable`, `reflect::before_step` / `after_step` / `can_pay` / `kh1_spell_cost` / `menu_sound`, `combat::Strikes::strike` / `trail_for_guard` / `has_trail` / `swing_start`, `shared::physics::kh2_controller` / `kh1_takes_air` / `kh1_velocity` / `kh1_air_attack_from_kh2`, `takeover::same_move` / `keyblade` / `carried` / `kh2_sideways` / `FormCast` / `POSE`, and the constants and statics only they used.
  - Kept on purpose: `DRIVER` (KH2's character data for the native row picks, abilities, growth and forms); `Strikes::strike_as` (native spells); Reflect's `intangible` (its native step); the left Keyblade's Final-only copy (`leftblade`, until Final Form is ported); `takeover.rs` not yet split into a helper module.
  - Not checked in game; `cargo test` not run here (no crate registry reachable); syntax checked only, so a missed use would show as a compile error. docs/shared-states.md (old controller section).

### 2026-10-29: Stage 876 (testing branch: KH1's jump / roll hand-off to the old KH2 controller deleted)
- The owner (chat, after t874: "just do all of them and report back when your done").
  - Deleted: `stylejump.rs` and `styleroll.rs` (KH1's own jump or roll started for KH2's controller and handed back to it), `takeover::style_air_start` / `style_roll_start` / `kh1_rise` / `inject`, the pose's roll override and every check of `stylejump::active` / `styleroll::active`. KH1's roll motion request is `takeover::KH1_ROLL_MOTION` (`0xDC`); KH1's glide check is `styles::kh1_now(Glide)`.
  - KH1's jump, glide and roll are KH1's own states' everywhere, as they were already on base Sora since t874. Not checked in game; `cargo test` not run here. docs/shared-states.md.

### 2026-10-29: Stage 875 (testing branch: the command and Attack borrows of the old KH2 controller deleted)
- The owner (chat, after t874: "just do all of them and report back when your done").
  - `combat`: the Attack hand-offs in `exec` (flight's air attack, KH1's roll, a KH1 move, KH1's borrows, the KH1-combo jump block), `spell_not_yet`'s casting / cut-window / Drive hand-offs, `command_step`'s borrow wait (now KH1's "can't" and a log line), its spell-during-cast block and its KH2-has section; the waiting and resumed command (`PENDING_COMMAND`, `command_waiting`, `spell_waiting`, `resume_command`, `take_chosen_command`); Reflect's chosen-while-busy path (an inline "can't"). `formmagic::spell_chosen` and `reflect::chosen` deleted.
  - `shared`: `BORROW` / `take_borrow`, `WANT_KH2`, `KH2_MOVE`, `lent`, `give_back`; `started` only sets the mode; `control` leaves every press to KH1's own control step except the move styles' (a borrow ever asked for is logged). `takeover::shared_frame`: a borrow still asked for leaves him with KH1.
  - A press that would have gone to the controller now goes to KH1's own row (as at t873). Not checked in game; `cargo test` not run here. docs/shared-states.md.

### 2026-10-29: Stage 874 (testing branch: the old KH2 controller's driving of Sora deleted)
- The owner (chat, after t873 tested: "everything is working continue"): the deletion, first area.
  - `takeover::takeover` (Sora's physics frame) kept its checks before KH2's part (a ride, an event, another body, KH1's area scripts), then now always: KH1's own states (`to_kh1_states`) when they don't have him yet; one of KH1's moves from a command taken on by KH1's states once KH1 lets him act (else KH1's own move, released); then `shared_frame` (a form on KH1's states, KH2's numbers, KH1's glide on Square; its borrows refused). Deleted (1,344 lines): the jump-under-KH2 fix-up, the old hand-off and take-back around KH1's moves (cut frames, air-cast landings, the form cast clock's hand-back), and KH2's controller frame itself (its step, collision through KH1's mover, flight start, its pose, footsteps, the give-back). So `takeover` never returns true: `FRAME_KIND` is never 1 and `kh2_has` is always false.
  - `shared::controller_cut_off` is now true on every build (off Steam nothing could run the controller any more; those builds have no native paths, so they lose KH2 combat until they get them, as agreed).
  - Items left with no use deleted: `form_cast_frame`, `spell_out`, `kh1_moving`, `object_top` and 17 statics (`FLY_PRESS`, `AFTER_CUT_LOG`, `FROZEN`, `NO_CARRY`, `HANDOFF_TICKS`, `HANDOFF_SLIDE`, `SLOPE_RUN`, `GROUND_JUMP`, `JUMP_PREV`, `PENDING_JUMP`, `JUMP_WATCH`, `LAND_NOW`, `COMBO_ON`, `SPELL_CUT`, `FLICKER`, `LAST_FROZEN`, `KH1_MODE`). What only those reached is left for the next versions (the borrow and resume paths in `combat` / `shared`, `stylejump` / `styleroll`, the controller-only checks in other modules, then `takeover`'s split).
  - Kept: KH2's controller data (`DRIVER`, made by `refresh_kh2_numbers`), which the native paths read for KH2's row selection, abilities, growth and form data.
  - Not checked in game; `cargo test` not run here (no crate registry reachable); unused-code warnings are expected until the next versions. docs/shared-states.md.

### 2026-10-29: Stage 873 (testing branch: the old KH2 controller cut off on Steam, the first step of deleting it)
- The owner (chat): "deprecate the old controller"; answer "2" (delete it outright), Final Form "we're going to transfer it over when ready"; plan agreed ("go ahead"): this cut-off first, then the dead code deleted one area per version, each tested before the next.
  - What still reached it on Steam at t872 (Checked (code)): the ini's `shared = off` (KH2's controller had base Sora whenever he stood, walked or ran), Drive Forms that don't run on KH1's states (Final; Valor / Wisdom when their slots didn't build), a command only it could run (a KH2 spell whose cast slot didn't build, KH1's Gravity / Stop / Aero with KH2 animation and no slot, Reflect when its native path can't), a command during KH1's jump / glide / roll hand-off, and the Attack hand-offs into it (KH1's roll, a KH1 move, KH1's spell cut, KH1's cut window, flight).
  - Now (`shared::controller_cut_off`, Steam): `shared::setting_on` reads the ini's `shared` as on (logged once if it says off); `form::choose` refuses a form that can't run on KH1's states (KH1's "can't", the Drive log line); `combat::command_step` refuses a command that would wait for a borrow (KH1's "can't"); the Attack hand-offs in `combat::exec` / `combat::spell_not_yet` are skipped, so the press goes on to KH1's own row as unmodded; and `takeover::shared_frame` turns any borrow still asked for into KH1 keeping him. Each refusal is logged (`shared::note_cut_off`, "not handed to the old KH2 controller").
  - Not changed: no code is deleted yet; off Steam (no native paths) the controller works as before. Final Form is refused on Steam until it is ported to KH1's states. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/shared-states.md (old controller section).

### 2026-10-29: Stage 872 (testing branch: KH2's allow-movement range as KH1 motion data: our level event 8.4)
- The owner (chat, after t871: "go ahead"; what-is-still-missing item 5 part 4).
  - KH2 (Checked (decomp)): range 41's handler sets the motion state word's bit 31 (`0x14040D810` case 0x29); the input step `0x1403A89A0` clears the stick unless a free range (bits 2..4, ranges 0 / 1 / 2) or bit 31 is on, so the stick steers a move only there.
  - Now: `kh2slots::kh1_tracks` bakes range 41 as our level event 8.4 (`kh1_data::slot::RANGE_ALLOW_MOVEMENT`), and on Steam `kh2slots::allow_movement` reads it with KH1's own reader (`kh1_level`: `Some` only while one of our slots plays, as before) instead of searching KH2's clip. Everything that steers by it changes source with it: a Drive Form's ground steering in its actions (`shared::physics::ground_inertia`: Wisdom's shots and casts) and its air steering (`form_air_steer`). Off Steam the clip as before. `slotcache::BAKE_FORMAT` 6.
  - The free ranges' half of KH2's rule was already KH1's own: KH2's free range is KH1's "may be left" (0.0) on our slots, which ends KH1's action flag, after which KH1's walk takes the stick.
  - Not changed: the bake's "allows movement all through" test for 8.1 (Wisdom's 181), and the old controller's KH1-spell-in-a-form clock (`takeover::FormCast`, KH2's controller path). Same frames as before. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.9.

### 2026-10-29: Stage 871 (testing branch: KH2's edge stop and ground hold as KH1 motion data: our level events 8.2 / 8.3)
- The owner (chat, after t870: "you can go on with part 3"; what-is-still-missing item 5 part 3).
  - KH2 (Checked (decomp)): its range handler `0x14040D810` sets the motion state word for range 3 (bit 5), 24 (bit 6), 29 (bit 23) and 30 (bit 24); the ground mover `0x1403B9090` keeps him on the ground (no "left the ground" call) while `0x1403BA9A0` (`+0x18C & 0x1000020`, ranges 3 / 30, the object entry's bit 0, `+0x120` bit 23) or bits 6 / 23 hold, and its edge stop sweeps the step for the floor in range 29. Range 30 is in none of Sora's or the forms' motions (Checked (files), t696).
  - Now: `kh2slots::kh1_tracks` bakes range 29 as our level event 8.2 (`kh1_data::slot::RANGE_EDGE_STOP`) and the "don't leave the ground" ranges (no gravity 3 / 24 on every clip, Wisdom's 181 included, or 29) as 8.3 (`RANGE_GROUND_HOLD`, `ground_hold_spans`). On Steam `kh2slots::edge_stop` (the ground swing's edge stop, `shared::physics::edge_stop`) reads 8.2 and `kh2slots::ground_hold` (the Ledge Carry, `shared::physics::ground_next`) reads 8.3 with KH1's own reader (`kh1_level`) instead of searching KH2's clip; off Steam the clip as before. `slotcache::BAKE_FORMAT` 5.
  - Same frames as before (start <= t < end, at KH1's time in the slot; through a blend the new motion's start, as KH2). Not changed: the edge stop's floor sweep itself (KH1's collision, `combomods::floor_gone`) and the Ledge Carry row. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.8.

### 2026-10-29: Stage 870 (testing branch: Wisdom's movement during its casts: KH2's push added to the move, KH2's velocity tick)
- The owner (chat, after t869, trace f11271..f11571 of Wisdom's ground Blizzard 59 / 59 / 60): "the numbers of how you move during a cast is off, go through the kh2 c decomp and compare"; no hook.
  - The trace (Checked (trace)): from frame 8 of 59 (12 of 60) Blizzard's push of its caster (50; 300 on 60) took the place of his whole move: the slide from before the cast was gone, the stick (held, 255 / 198) did nothing through 59's allow-movement range from 10, and the frame the push fell below 1 (24 on 59, 45 on 60) he went from about 1 to 8.6 a frame at once.
  - KH2 (Checked (decomp)): the physics step `0x1403B89A0` adds, each frame, the held velocity move, the root, the push (`0x1403D4570`: direction x speed x frame, then x 0.9, ended below 1) and the rest; its velocity step runs on a clock (`+0xBA8`, 0 at an action change, down by the frame scale, below 0 the step with dt 2 and the clock + 2), so it runs every second 60 Hz frame and its velocity is held. The input step `0x1403A89A0` clears the stick unless a free range (0 / 1 / 2) or range 41 is on. The ground cast start `0x1403C5170` only sets the motion. The free state's move `0x140422350` takes the inertia path (`0x1403B7AD0`: with the stick, velocity = target x (1 - acceleration) + velocity x acceleration; without, x brake; acceleration and brake once a step) on range 42, which Wisdom's 0 / 1 / 56 / 59 / 60 have all through (Checked (files), `P_EX100_MAGF.mset`; 59: range 41 from 10, 60: from 34).
  - KH1 (Checked (decomp)): its physics step `0x14029A3C0` calls its push `0x1402C7070` when `+0x88` bit 0x40 is set, which writes the push over the frame's move vector: KH1's push replaces the move, KH2's adds to it.
  - Now: `hitreact::script_push` with a Drive Form's ground inertia on the ground (Wisdom) hands the push to `shared::physics::inertia_push` instead of KH1's push, and `ground_inertia` adds it to his move each frame on KH2's curve (0.1 of the distance a frame, x 0.9, ended below 1). `ground_inertia`'s acceleration and brake now run on KH2's clock (`GROUND_TICK`: the step on the first frame of a fresh ground velocity and every second 60 Hz frame, the velocity held between) instead of easing every frame by `dt / 2` powers. The stick rule and the target speeds (walk 2, run 12, run from 0.8 tilt) were already KH2's. No hook.
  - Effect: all of Wisdom's ground movement now changes speed every second frame, as KH2's does; during a ground cast the push is on top of his slide and steering, not in place of them.
  - **Assumptions**: the ground cast keeps the action it started from (no clock restart on a cast; inferred from the cast start setting only the motion); a push still running when he leaves the ground is dropped (Blizzard's starts on the ground on this path). KH1's push is unchanged everywhere else (base Sora, KH1's own reactions, the air). Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-drive-forms-native.md §8.1, docs/kh1-magic-native.md §8 (t821 note).

### 2026-10-29: Stage 869 (testing branch: KH2's no-gravity hold as KH1 motion data: our level event 8.1)
- The owner (chat, after t868 tested working: "go with the next step"; item 5 part 2).
  - Design, refined from the plan in docs/kh1-anim-baking.md §14.3: our level events are read with KH1's own reader where they are used (`kh2slots::kh1_level`, t868), not copied into free bits of `+0x1CC` by a patch on KH1's pose pass. KH1's pass writes `+0x1CC` only on the plain pass (Checked (decomp), t868), so bits there would hold the old motion's values through a blend, while the reader at KH1's time (held at the new motion's start through a blend) gives what KH2 reads (`0x1403C6740`, Checked (decomp)). No free bit is needed, so none is written.
  - Now: `kh2slots::kh1_tracks` bakes KH2's no-gravity ranges (range ids 3 / 24) into our slots as our level event 8.1 (`kh1_data::slot::RANGE_NO_GRAVITY`), left out for a clip that allows movement all through (Wisdom's shots, `no_gravity_spans`, as before). `kh2slots::no_gravity` reads 8.1 with KH1's reader on Steam instead of searching KH2's clip; everything that follows it changes source with it: the air step's hold and its cut (`shared::physics`), the root travel in the hold (`air_root`), the forms' travel outside it (`air_root_form`), the forms' lift-off (`lifts_off`) and the end of the rise start (`rise_start`). Off Steam the clip is read as before. `slotcache::BAKE_FORMAT` 4.
  - Not changed: the root travel itself (KH2's clip root, a separate item), `rise_start`'s first-frame lookup on the motion not yet started, `ground_hold` (no gravity or edge stop: part 3), and `shared`'s KH2 hover on KH1's own air cast animation (KH1's own motion, no slot of ours to carry 8.1).
  - Same timing as before: the same time field and the same frames (start <= t < end). Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.7.

### 2026-10-29: Stage 868 (testing branch: KH2's invulnerable ranges as KH1 motion data: our level event 8.0 read by KH1's reader)
- The owner (chat, what-is-still-missing m30 / m31, item 5 part 1, answer "(a)"): KH2's motion ranges onto KH1's motion events, the invulnerable ranges first, 1:1 by level rather than a one-off event.
  - KH2 (Checked (decomp)): every motion update clears the range bits (`0x1403C83B0` from `0x14040E5C0`) and sets them again for each range the clip's time is in (`0x14040E740` -> `0x14040D810`, range 27 -> `+0x18C` bit 22), at a blending motion's start time (`0x1403C6740`: `+0x4C` while `+0x48` > 0). So the invulnerability lasts exactly while the time is in the range, through the blend, and ends on the update after a cut.
  - KH1 (Checked (decomp)): its pose pass (`0x140391C70`) reads only 0.0, 1.0..1.3, 5.0, 7.0 with its level reader `0x140394770`; the blend pass (`0x140392560`) reads none, so `+0x1CC` keeps the old motion's levels during a blend; its edge pass fires only types 4 and 6, and none while a blend holds the time. "No damage" is read from the actor's flags (`0x1000080`) in six places (`0x140291920`, `0x140297FE0`, `0x14029BBD0`, `0x1402A3BA0`, `0x1402B4060`, `0x1402BEEE0`).
  - Now: `kh2slots::kh1_tracks` bakes range 27 into our slots as a level event of ours, 8.0 (`kh1_data::slot::KH2_RANGE` / `RANGE_INVULNERABLE`; vanilla KH1 never asks for type 8). `kh2slots::kh1_level` asks KH1's own reader for a level of ours on the motion KH1 plays (`+0x1D4`'s table at slot `+0x168`) at KH1's time (`+0x16C`, held at the new motion's start through a blend), only on our slots; reader bytes `48 89 5C 24 10` checked first (Checked (exe)). `invulnerable_step` (each frame he is posed, as before) holds KH1's no-damage flag while 8.0 is on (timer 2.75, re-written each frame, marked by its fraction) and drops it the first frame it is off; a longer KH1 timer of its own is kept. The KH2 clip lookup and t860's frames-left timer are gone. `slotcache::BAKE_FORMAT` 3 (the slots rebuild once).
  - Fixes against t860: a range from frame 0 (Aerial Recovery, Counterguard) now holds through the blend into the move; a cut move no longer leaves the timer running.
  - Not changed: no free bit of `+0x1CC` is written yet (part 2 decides that after the free-bit check). Timing as t860: the flag is set in the pose pass, so a range's first frame counts from KH1's next hit pass (**Assumption**: Steam's actor order think, physics, matrix, animation, traced on Epic). Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-anim-baking.md §14.6, docs/kh1-intangible.md.

### 2026-10-29: Stage 867 (testing branch: KH2 Cure's heal: KH2's formula with max MP as Magic)
- The owner (chat), after the max MP check: "(b)". Cure's heal (`formmagic::heal`) is KH2's own formula (`combat_core::damage::compute_healing`): kind 6 (Cure's rows 708 / 709, Checked (files)) power × Magic / 100, kind 5 the target's max HP × power × Magic / 10000, rounded up, with the caster's max MP (stats `+0x48`, what KH1's attack constructor `0x1402B2DC0` gives its stat source 1, Checked (decomp)) as KH2's Magic. t828's share of max HP × `magic_factor` and `CURE_SHARE` are gone. Cure level 1 heals 4 HP a max MP point on Sora, 2 on Donald and Goofy. Assumption: stats `+0x48` is max MP (KH1's party refill `0x1402AD0B0` raises MP `+0x44` toward it). Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 866 (testing branch: KH2's blow ends the Magnet's hold on a flier; t865's travel drop undone)
- The owner (chat): "there is some visual issues but they are getting hit properly, its in the log" (t865, Magnet Debug on).
  - What the log showed (Checked (log)): t865 dropped the knock-back travel on every held flier (36 a frame, 0x4C / 0x4D), and KH1 moved them by our suction only, as meant. But while 0x4C / 0x4D played, the distance from each flier's body centre (KH1's bone point 0x8003, what the suction aims from) to the Magnet jumped 41 → 249 → 265 → 305 a frame apart while its place moved about 25: the body drifted up to about 270 from the actor's place, about the travel dropped (actor 0x…9370 f815..f843; the Magnet's centre, worked out from the 0x4E frames of two other fliers, sits about 35 from its place at f829 while the body point is 305 away). So the model slid off its place, and the suction's direction flipped each frame: the visual issue. **Assumption**: KH1's pose keeps the motion's travel that `+0x200` doesn't carry off.
  - KH2 (Checked (decomp), Steam KH2): `0x1403D2FE0` case 4 (a blow) plays the blow motion (0x12 / 0x10), sets the blow speed and the enemy's blow action through `0x140409D30` (4, or 0x21 / 8 for its flying kinds), whatever its action was: a blow ends the catch (actions 0xE / 0xF) for every enemy. Magnet Burst's 1901 / 1903 are blows.
  - Now `combat::knocked_back` (KH1's knock-back `0x1402B77D0`, every enemy knock-back) first runs `magnet::knocked_back`: a flier held in 0x2C goes back into its own state (`before`, KH1's class 2) before KH1's knock-back runs, so KH1 plays its flier knock-back in place with its own travel, as in vanilla KH1, and the hold is over. Held grounders unchanged (KH1's knock-back already takes them out into state 2). t865's `drop_motion_travel` is gone; the Magnet Debug line keeps the `+0x200` column. Lines "blown out of the hold" (Magnet Debug) / "blown out after" (Combat).
  - Not checked in game; `cargo test` / build not run here. docs/kh1-hit-reactions-native.md §17.

### 2026-10-29: Stage 865 (testing branch: a held flier isn't moved by KH1's knock-back travel)
- The owner (chat): "recorded some traces and logs" (t864, Magnet Debug on; traces f39988..f40815 have Sora's columns only).
  - What the log showed (Checked (log), 1,486 Magnet Debug lines): KH1 moved every held enemy by exactly the sum we expected each frame (the "moved" column matches "expected" frame for frame), so nothing outside KH1's physics step moves them. The push (`+0x88` bit 0x40) was never on. The one-frame push `+0x120` was small (KH1's body separation, up to 10). The cause is the vector at `+0x200` (added while `+0x160` bit 4): it is on only while a held flier (state 0x2C) plays motion 0x4C / 0x4D, KH1's knock-back in place from Magnet Burst's 1901 / 1903 blows (t863), and it grows to about 80 a frame (actor 0x…8560 f39764..f39788: 36 → 80, turning about 17° a frame with the facing our step sets), on top of the suction's 28. That is the "super fast". Held grounders (0x2B) never had it on.
  - Now `magnet`'s step sets that vector to 0 while the enemy is held (`drop_motion_travel`), after KH1's motion update and before KH1's physics step adds it; the knock-back motion still plays and the bit is left as KH1 set it. KH2's catch step (`0x14040F8E0`, Checked (decomp)) is the held enemy's only movement. The Magnet Debug line shows what was dropped. At the let-go nothing is dropped any more: the flier goes back into its own state and KH1's reaction plays on as KH1's.
  - **Assumption**: `+0x200` is the motion's own travel (written by KH1's motion update each frame; its writer not found in the decomp), so zeroing it only stops that one add. Not checked in game; `cargo test` / build not run here. docs/kh1-hit-reactions-native.md §17.

### 2026-10-29: Stage 864 (testing branch: a Magnet Debug log for enemies Magnet Burst holds)
- The owner (chat, after t863): "it seems to improved but now flying enemies go super fast inside of the magnet pull", then "make it and add it as a toggle in the logs section on debug, go ahead and check magnet.rs". Diagnostic only: nothing in how the Magnet moves anything changes.
  - What the t863 log showed (Checked (log)): every catch is "orbit 0 (between 0 and 0), swirl 20". That is KH2's own data: ATKP 1900 in `00battle.bin` has 0 / 0 at `+0xC` / `+0xE` and 20 at `+0x10` (1901: 20 / −2, as logged) (Checked (files)), so KH2's suction pulls to the centre itself at up to 20 a frame, with the swirl at 20. Held ground enemies close in (647 → 122 in 30 frames); held fliers (state 0x2C) move away (164 → 622, 189 → 625, 152 → 578), which our step alone can't give: something outside it moves them. Not found yet.
  - KH1's physics step `0x14029A3C0` adds to the state's step (Checked (decomp)): the push (`+0x88` bit 0x40, `0x1402C7070`: direction `+0x3F0` × frame scale × speed `+0x3FC`), the vector `+0x200` while `+0x160` bit 4, and the one-frame push `+0x120`; then KH1's mover. KH1's knock-back in place on a class-2 state (`0x1402B8CC0`) plays 0x4C / 0x4D, or without them `0x1402B9230`, which ends in KH1's flinch push `0x1402B7E60` (our `knockback::flinch_push`) (Checked (decomp)).
  - New log category **Magnet Debug** (`LogCat::Magnet`, ini `log_magnet`, off by default, on the debug menu's Logs page). With it on, `magnet`'s step writes one line per held enemy per frame: its state, motion (and action flag), place, distance, our displacement, the push / `+0x200` / `+0x120` it will get this frame with their on bits, their sum, and last frame's expected move against how far it really moved; plus a line at each catch and let-go. At most 20,000 lines a session. Off: one atomic load per held frame.
  - Review of `magnet.rs` (no change made): the radius / swirl read the right ATKP fields; a re-catch on every hit restarts the hold's frame count and the hurt motion (KH2's catch also restarts motion 20 / 21 on every hit); KH1's push is cleared only at the catch, so a push KH1 starts while held (the flinch push above) adds to the suction; the step's × 2 of KH2's (`0x14040F8E0`) is still the 30 Hz-tick assumption.
  - Not checked in game; `cargo test` / build not run here (no crate registry or Rust target download reachable). docs/kh1-hit-reactions-native.md §17, docs/kh1-debug-menu.md.

### 2026-10-29: Stage 863 (testing branch: a held flier stays a flier: the Magnet's second KH1 state, class 2)
- The owner (chat, after t862): "do you know why flying enemies are getting knocked down and start falling?", then a log with Combat on and a trace (the trace has Sora's columns only).
  - What the log showed: the five flying enemies were caught from KH1 state 4 (flags 2: KH1's class 2), the others from 0 / 2. Magnet Burst's 1901 (reaction 9, a KH2 blow 20 / −2, Checked (files) `00battle.bin`) then landed with KH1's kind 4 on each; on the fliers, held in t862's 0x2B (class 1, an air state to KH1), KH1's knock-back `0x1402B77D0` took its airborne branch: requests 0x50 / 0x51 and a hop of 20 into state 2 (`0x1402AF960`), and `knockback` flew it as KH2's blow with gravity ("KH2 blow on … (KH1's airborne knock-back)", f1960..f1963): they fell. On a class-2 state KH1 takes the other branch, `0x1402B8CC0`: its knock-back in place (0x4C / 0x4D, no state change) (Checked (decomp)). The grounders were launched out of the hold by the same hit ("fell back to its launch"), which is KH2's own rule (its case 4 blows them out of the suction, `0x1403D2FE0`, Checked (decomp)).
  - Now `magnet` has a second state, 0x2C (flags 2: class 2), same step and transition, also written each Sora physics frame (never registered nor switched on by number, Checked (decomp)). An enemy caught from a class-2 state (`0x1402B1AE0`) is held in 0x2C, so KH1's hit pass gives a held flier its fliers' reactions; the let-go puts it back in its own state (t862's answer 4). The held motion is asked again only once nothing with KH1's action flag plays (a knock-back KH1 gives a held flier plays out first). t862's assumption that KH1's fliers are its class-2 states: Checked (log), state 4.
  - Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-hit-reactions-native.md §17.

### 2026-10-29: Stage 862 (testing branch: KH2's Magnet catch as a KH1 movement state of its own)
- The owner (chat, from the native chat's list: make "Magnet Burst's pull" native; asked how first, then "1. (a) 2. (a) 3. (a) 4. (a) go ahead": the enemy plays its own KH1 air-hit motion held, the radius on KH1's random number, the let-go by the Knock-back Numbers row, fliers back into their own state).
  - Before: an enemy the Magnet window (ATKP reaction 5) hit went into a list of ours (`bodyhits::Live::caught`), and every frame `takeover::kh2_hits` → `combat::Strikes::native_pull` / `magnet_push` added KH2's suction into KH1's one-frame push `+0x120` while the enemy's own state, gravity, motion and think went on under it; KH1 also played its own reaction for the hit (the template row's kind); the radius came from the actor's address. Running along, not native.
  - KH2 (Checked (decomp), Steam KH2): the reaction dispatch's case 5 (`0x1403A8DE0`, `0x1403D2FE0`, `0x1403D4510`) calls the catch `0x14040FBF0 (enemy, attack, dot(forward, hit direction) < 0)` on every hit: the attack kept (`+0xCF8`), radius by KH2's random number between ATKP `+0xC` / `+0xE`, swirl `+0x10`, motion 20 / 21 (front / behind) with no blend, turned to the attack (`0x14040FA50`, 21 away), action 0xE (0xF by an object flag); its step `0x14040F8E0`: along the 3D line to the attack at `(distance − radius) × 0.1` capped at ±20, plus the flat quarter-turned line × swirl, no gravity, × 2 returned (**Assumption**: its 30 Hz tick); turned again every frame while the attack lasts (`0x14040FB10` / `0x14040FB60`); with the attack gone 0xE goes to action 5, 0xF to its own recovery (motion 4, blend 8).
  - Now (new `magnet`, Steam only): KH1 movement state 0x2B (never registered nor switched on by number, as Quick Run's 0x2A; Checked (decomp)), flags 1 (KH1's air collision class), our step and transition in KH1's table (`0x142DB3840`), written each Sora physics frame into an empty slot or ours. `bodyhits::hit` (KH1's contact, `probe::hit`) runs `magnet::catch` on every Magnet hit: not on an actor KH1 takes no reaction on (`+0` & `0x1000088`, as `0x1402B7B90`), one outside KH1's physics (`0x140292010`), or the big and immovable (`combat::immovable`, `Body::fixed` as before); the record's kind becomes 1 (KH1's enemy hit pass `0x1402B8E10` has no reaction for it, its damage and a physical hit's prizes stay, `0x1402B7B90`, Checked (decomp)); radius by KH1's random number (`0x14031BA30`, `kh1picker::random`); KH1's state change `0x1402B1960` into 0x2B; KH1's air flinch request 0x4E (front) / 0x4F (behind, KH1's own rule on the record's `+0x10`) with the action flag and no blend; turned with KH1's facing set `0x140297E10`; momentum, push and any KH2 flight dropped. KH1's physics step `0x14029A3C0` calls our step: KH2's suction from KH1's body centre (`0x14029EB20`'s 0x8003) to the Magnet's KH1 attack object's own place (`+0x10`) × KH1's frame scale, the turn each frame, the motion asked again when another replaces it; KH1's mover and walls move it. The let-go when the window stops being built by KH1's pass (`bodyhits::magnet_centre`, ≤ 2 frames after): from a class-2 state (`0x1402B1AE0`, KH1's fliers) back into it with KH1's idle queued; else KH1's fall from rest (`0x1402AF8F0 (actor, 0)`) with no momentum, KH2's no-speed blow on it at Knock-back Numbers KH2 or Scaled (`knockback::fly_blow`), KH1's fall 6 queued (blend 8). Gone: `Strikes::pulled` / `pull` / `native_pull` / `magnet_push`, `bodyhits::catches`, the radius by address. New hooks: none (prologues of the KH1 functions it calls checked, Checked (exe)).
  - Correction to the plan: the KH2 Magnet spell never pulled (its KH1 objects carry no Magnet catch); it is not part of this version.
  - Not checked in game. **Assumptions**: KH1's enemy think leaves the state alone and acts on nothing while the action-flag hurt motion plays; KH1's class-2 states are KH2's 0xF enemies; KH2's re-catch runs on every hit while held. The fallback sweep (`kh2_hits` when `bodyhits` isn't in, not Steam) no longer pulls. `cargo test` not run here (no crate registry reachable). docs/kh1-hit-reactions-native.md §17.

### 2026-10-29: Stage 861 (testing branch: the Keyblade guard on KH2's own shapes through KH1's builder and meeting; `hurt`'s guard check off)
- The owner (chat, closing threads keyblade-guard-facing-native and invulnerable-frames-native to finish them in chat; answers "(a) (a) (a)": the guard on KH2's shapes through KH1's builder like Reflect's barrier, the repel's protection 1:1, attacks that are an enemy's body follow the attack-against-attack rule).
  - Before: the guard press 173 (ATKP 161), its repel 42 and Guard Break 165 (162) were KH1 guard objects of clash type 4, one sphere per shape (`combat::guard_step`), and `hurt` also judged every hit reaching his body against the guard (`kh2slots::keyblade_guard`, the hit record's push direction). Type 4 meets only enemy attacks of types 0, 3 and 5 in KH1's pairing table (Steam `0x140528CB8`, Checked (exe), t813), so `hurt` was what stopped the rest.
  - Now (Steam, with KH1's builder and meeting hooks in, `combat::keyblade_guard_native` = `barrier_native`): the Keyblade's guard objects are clash type 3 with no one-hit bit, as the barrier's (t813), and stand on KH2's own guard shapes through KH1's entry builder (`bodyhits::guard_shapes`; `guard_entries` holds their result word at "clashed" so they never meet a body). KH1's meeting (`recoil`'s hook) notes each enemy attack one meets after KH1's two directional tests pass with the window's ATKP arc (`combat::keyblade_guard_meeting`); `guard_step` then runs KH2's guard reaction by refact self (`combomods::guard_repel`: the repel and its 80 frames at Recoil KH2, KH1's block at Recoil KH1), once a frame. KH1's own halves stop the enemy's melee attack (its owner's clash record) or turn a shot back. `hurt`'s Keyblade-guard check runs only where the native path can't (not Steam, hooks refused). The body guards (weapon 0) are unchanged (type 4, `hurt`'s `body_guard`).
  - KH2's direction (Checked (decomp)): `0x1403D1F40` dots the owner's facing (turned by the ATKP angle) with `YS::ATTACK` vtable `+0x10` = `0x1403D1430` (vtable `0x1405C5E78`, Checked (exe)): from the attacker's owner to the guard's owner, height dropped. KH1's `0x1402B4640` (Checked (decomp)) uses the line between the two owners' `+0x50` with the height kept: same rule, **known gap** for attackers far above or below. The hit record's push direction `hurt` used is neither.
  - Not checked in game. **Assumption**: KH1's attack-against-attack test runs before a body hit from the same attack lands (a hit that reaches his body now is taken, the owner's (a)). `cargo test` not run here (no crate registry reachable). docs/kh1-roll-guard-native.md §8.

### 2026-10-29: Stage 860 (testing branch: KH2's invulnerable ranges on KH1's no-damage timer)
- The owner (chat, as t861).
  - KH2 (Checked (decomp)): motion range 27's start sets `+0x18C` bit 22 (`0x14040D810` case 0x1B, on the motion block at `+0x158`), which the actor's invulnerable test reads (vtable `+0xB8`, `0x1403D65A0` via `0x1403D3570`): a hit lands as kind 4, as on the timer (t858).
  - Now `kh2slots::invulnerable_step` (each frame he is posed, from `kh2slots::frame`): when our slot's range 27 opens (Explosion 32..64, Aerial Recovery 0..16, Counterguard 0..20, Finishing Leap 24..38, Checked (files)), KH1's no-damage timer gets the frames left in it (`combomods::kh1_no_damage`), marked with a quarter frame less (`RANGE_MARK`) so it can be told from KH1's own whole-frame timers; a longer KH1 timer already running (a reaction's 30) is kept. The move left before the range ends: our timer ended (**Assumption**: KH2 drops the bit with the motion). `kh2slots::invulnerable` and its `hurt` check are gone.
  - Not checked in game; `cargo test` not run here. docs/kh1-intangible.md.

### 2026-10-29: Stage 859 (testing branch: KH2's Quick Run invulnerability on KH1's no-damage timer)
- The owner (chat, as t861).
  - KH2's Quick Run start `0x1403A9830` stores `fmab +0x1C` in `+0xD70` (`0x1403D7E40`, Checked (decomp)). `quickrun::start` now gives KH1's no-damage timer those frames (0 at LV1 ends a roll's timer, as KH2's store of 0 does) in place of clearing the roll's flags; `quickrun::invulnerable` and its `hurt` check are gone. Not checked in game; `cargo test` not run here. docs/kh1-intangible.md.

### 2026-10-29: Stage 858 (testing branch: KH2's dodge roll invulnerability on KH1's own no-damage timer)
- The owner (chat, as t861).
  - Research (Checked (decomp)): KH2's hit on an invulnerable actor still lands (`0x1403D1730`: the actor's vtable `+0xB8`, `0x1403D5310` / `0x1403D65A0`, true while `+0xD70` > 0, `+0x18C` bit 22 or `+0x126` bit 0, makes the result kind 4 instead of 1; the record, the contact and the attacker's side still run). KH1's no-damage timer (`0x140297F30`: `+0xC`, flag 0x80) does the same (kind 4 at the contact, no damage `0x1402BEEE0`, no reaction but a clash `0x14029BBD0`); KH1's untouchable flag 0x40 (left out of the hit pass) is the scripts' (event op 0x2B / 0x2A, `0x14028E6E0`), not the roll's. Sora's think `0x140297FE0` counts `+0xC` down by the frame scale `0x14233FBDC` (60 Hz vblanks) and clears 0x80 / 0x100000 at 0; KH2's `0x140717480` is 1.0 a 60 Hz frame too. So KH2's invulnerability is KH1's timer, not 0x40, and `hurt`'s throw-away (no effect) was neither.
  - Now `combomods::kh1_no_damage` (frames, both flags) and `kh1_no_damage_on`; `roll_press` sets KH1's roll timer to the level's `dodge_roll_star_time` (10 / 16 / 24 / 32) in place of undoing it, as KH2's `0x1403A81F0` stores it in `+0xD70`. It runs on if the roll is left early, as KH2's timer does. `roll_invulnerable` and its `hurt` check are gone; `hurt` doesn't note a KH1 knock-back while the timer runs (KH1 starts none).
  - Not changed: the borrow's invulnerability (`reflect::intangible`, flag 0x40) for forms and KH2's controller. Correction to the chat round: the repel's 80 frames were already in (`combomods::guard_repel`, Recoil KH2); the 180 for ATKP 2012 / 2057 (`0x1403A73F0`) can't happen on KH1's enemies, whose attacks have no KH2 ATKP. Not checked in game; `cargo test` not run here. docs/kh1-intangible.md.

### 2026-10-29: Stage 857 (testing branch: KH2's own attack moves follow the Physics row again; t856 undone)
- The owner (chat, after t856): "I still want it to use the physics row actually"; asked what Magnet Burst's dip at Physics KH1 should do: "Undo t856, keep the dip".
  - t856's code taken out: `shared::physics::{kh2_move_hop, KhMove, MOVE, MOVE_LAST, move_kh2, kh2_move_left}`, `kh2_hop_as`'s move flag and the `air_step` / `kh1_air_attack_started` parts that read them; `combat::exec` calls `kh2_hop` again; the Physics row's help text (`0x7F00_011A`) back to t855's. `shared.rs`, `combat.rs` and `config.rs` are t855's again; t853..t855 (`mark_hop`, `HOP_CUT`, `form_left_in_air`) stay.
  - So KH2's moves take the Physics row's physics (t621's rule): at Physics KH1 Magnet Burst's hop of 14 is KH1's and dips about 7 under its start before its hold at frame 16 (trace f2285, Checked (trace)); at Physics KH2 it ends about 2 over. Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-magic-native.md §4i.

### 2026-10-29: Stage 856 (testing branch: KH2's own attack moves on KH2's physics whatever the Physics row)
- The owner (chat, t855 trace `trace_1793254014_f2285`: "same issue on magnet burst"; asked (a) keep t621's rule, (b) KH2's moves on KH2's gravity apart from the Physics row, (c) a toggle: "(b) it should be move independent from the physics setting, in this situation it looks more like a bug than a design choice").
  - What the trace showed: not t854's cut hold (KH1's own second air swing 0xCD has none). Base Sora at Physics KH1, Magnet Burst 197's row hop of 14 was KH1's (`kh2_hop`, "KH1's gravity (physics KH1)"): 0.48 a frame, peak 94.5 five frames in, 73.0 at its hold at frame 16, 7.4 under its start (f2016..f2033; again f2196..f2213). On KH2's gravity (0.41, 8.3 frames up, as Valor's 183 at f1923..f1940 of trace f2082) it ends about 2 over. Checked (trace).
  - Now `combat::exec`'s KH2 row hops go through `shared::physics::kh2_move_hop`: KH2's arc for the hop whatever the Physics row, and `air_step` takes KH2's physics (hop, hold with KH2's clock, the fall's restart at the hold's end, KH2's air drift) while that move has him (`MOVE`, `move_kh2`: the hop's frame, KH1's slot still on the cut move or not yet on ours for up to 4 frames, then while KH1's slot plays that KH2 motion). When it ends at Physics KH1, `kh2_move_left` hands him to KH1's air step (from rest if the hold had him last, as t855's revert). A KH1 air attack (`kh1_air_attack_started` with no KH2 hop that frame) or any other `kh2_hop` (recoil, Aerial Recovery, hit reactions) ends it. Replaces t621's rule for these moves; KH1's own moves, the jumps, the Aerial Dodge, the reactions and air casts (Air Physics row) unchanged. The Physics row's help text (`0x7F00_011A`) says so. KH2's moves in ACTION_JUMP with the row hop `0x1403D78D0` and step `0x1403CB360`: Checked (decomp), t617. **Assumption**: every row `combat::exec` hops for is one of KH2's moves (they are KH2's row selection's runs). Not checked in game; `cargo test` not run here (no crate registry reachable). docs/kh1-magic-native.md §4i.

### 2026-10-29: Stage 855 (testing branch: a Drive Form ending in a held air move falls from rest)
- The owner (chat, after t854 "working"; trace `trace_1793253630_f2082`: "if you return in the air during a finisher in valor form you fall extremely fast").
  - Cause: Valor's 183 holds him from its frame 16 (no-gravity range); through the hold KH2's arc clock runs on (KH2's ACTION_JUMP step `0x1403CB360`) and KH1's arc fields were written from it at his held height, so by f1975 they read time 53, base 411.9. The revert (f1976) hands base Sora to the Physics row's physics (KH1 in this trace), and KH1's air step took those fields as they were: a fall 54 frames old, 16.2 down at once and 19.6 a frame at the ground 8 frames later (147.8 to 0). Checked (trace).
  - Now `form::native_revert` in the air calls `shared::physics::form_left_in_air`: held at his last air step, KH1's arc is set to a fall from rest where he is (sideways speed kept), as KH2's restart at a hold's end does (`0x1403B4780`, vertical speed 0 after a hold, Checked (decomp), t653); not held, KH1's fields are already his arc; KH2's arc is dropped either way (`off_air`), so at Physics KH2 the arc goes on from that fall. **Assumption**: KH2's revert in the air ends the hold through that same restart (the revert's own think not traced). Not checked in game; `cargo test` not run here (no crate registry reachable).

### 2026-10-29: Stage 854 (testing branch: the move a hop cuts holds nothing until KH1 plays the new one)
- The owner (chat, t853 trace `trace_1793253305_f1084`: "same issue is still happening").
  - Correction to t853: its cause was wrong. The hop of 184 out of 183's Finishing Plus window goes through `kh2_hop_as` as well (the log at f981 has both "KH2's hop on KH1's air state: 14 high" and "KH1's air attack out of KH2's motion"), so `HOP_FRAME` and the hold flag's clear were already there; t853's extra marking in `kh1_air_attack_started` changed nothing for this move. Checked (log).
  - Cause: the hop is set at f981, but the next air step (f982) still finds KH1's slot on 183 (frame 53+, inside its no-gravity range 16..end; KH1 plays 184 from f983, "KH1 plays ours, KH2 motion 184"), outside `HOP_FRAME`'s one frame. It held him (moved 0, the trace's frame 982) and set the hold flag, and at f983 184's frame 0 has no hold, so the hold's end restarted his fall from rest (`kh2_fall_at`, KH2's `0x1403B4780`): 134.3 down to 82.0 by 184's own hold at 16. Checked (trace, log).
  - Now `mark_hop` (from `kh2_hop_as` and `kh1_air_attack_started`) also keeps the cut move (`HOP_CUT`: its KH2 motion, KH1's time in it, the frame), and the air step holds nothing for it while that same motion still plays without starting over, at most 4 frames (`cut_still_playing`). KH2 starts the row's motion and its hop in the same think (`0x1403D78D0` -> `0x1403CBB00`, Checked (decomp), t759), so the old motion never holds the hop. 184 should then rise its 14 out of 183 as 183 does out of 182 (trace f928: 135.8 up to 149.8, then 134.3 at its hold). Not changed: the rising rows' bridge (`rise_start`), the Drive's arrival, 183's own dip between its hop's apex and its hold at 16 (KH2's row hop and range, t739). Not checked in game; `cargo test` not run here (no crate registry reachable).

### 2026-10-29: Stage 853 (testing branch: a KH1 air row's hop isn't held by the move it cuts either)
- The owner (chat, two videos of Valor and trace `trace_1793252677_f26703`: "when sora does another finisher after he does the first one, inbetween both he drops a little bit which is not how kh2 does it").
  - Cause: t759's fix only covered hops started by `kh2_hop_as` (Aerial Combo KH2). With Aerial Combo KH1 (this trace's setting), Valor's 184 out of 183's Finishing Plus window runs KH1's air swing row, whose start sets its own hop (14) in KH1's command step (`combat::command` -> `shared::physics::kh1_air_attack_started`), which marked neither `HOP_FRAME` nor cleared `HELD_LAST`. At f26573 the hop is in KH1's arc fields (time 1, 8.28) but KH1's slot still plays 183 at its frame 53, inside its no-gravity range (16..end), so the air step held him at 98.88 and set the hold flag; at f26574 (184's frame 0, no hold) the hold's end restarted his fall from rest (`kh2_fall_at`, KH2's `0x1403B4780`) and the hop was gone: 98.88 down to 46.64 by f26589, then 184's own hold at its frame 16. Checked (trace).
  - Now `kh1_air_attack_started` does what `kh2_hop_as` does when a KH1 air attack starts: `HOP_FRAME` (the cut move holds nothing this frame) and `HELD_LAST` cleared (no fall restart over the new hop). KH2 starts the row's motion and its hop in the same think (`0x1403D78D0` -> `0x1403CBB00`, Checked (decomp), t759), so 184 rises its 14 out of 183 as 183 did out of 181 (f26519: 100.4 up to 114.4, back to 98.9 at its hold). Not changed: 183's own drop between its hop's apex and its hold at 16 (KH2's row hop and range, as t739). Not checked in game; `cargo test` not run here (no crate registry reachable).

### 2026-10-29: Stage 852 (testing branch: the left Keyblade's keychain hung as KH2's left one)
- The owner (chat, t851 screenshot and video in Olympus: "the chain on the left keyblade is flying up", "this happens during the idle and walking", "actually running is fine, just walking and idle").
  - Cause: `pose::Baker::holder_poses` starts from the main bake's pose, whose 283..292 are the right Keyblade's keychain (hung by `secondary` from KH2's right weapon motion). `kh2_hold` carries the bones under 282 with the left Keyblade, so the left chain kept the right chain's shape about its own blade. Offline (Valor, the owner's files): the left chain (283 to 291) 126 degrees off KH2's left chain (weapon bones 3 to 11) on average in motion 0 (idle), 84 in 1, 63..64 in 2 and 3, 123 in 4; at idle frame 0 it pointed (0.69, 0.57, 0.44) (up) where KH2's hangs (-0.36, -0.93, -0.05). Checked (files, offline).
  - Now: after the left hold, `secondary` hangs 283..292 from KH2's left weapon motion (chain only), as it does the right one from the right weapon motion. Offline: within 1 degree of KH2's left chain in every motion checked. `holder_key` adds "left chain": the holder motions rebuild once. Not changed: the right chain, KH1's swing (`jiggle`) for chains KH2 doesn't pose.

### 2026-10-29: Stage 851 (testing branch: Valor's Keyblades held in the palm, placed from the middle knuckle)
- The owner (chat, t850 screenshots of Valor in Olympus: "hes still not gripping correctly").
  - Cause: `pose::kh2_hold` carried KH2's weapon bone 2 from KH2's palm onto KH1's with the whole of `kh1_bind[palm]⁻¹ × kh2_bind[palm]`, which also carries the two models' palm bones' places apart. KH1's palm bone sits on the middle knuckle (113 at its origin, the wrist 11.0 behind); KH2's sits mid-hand (the wrist 5.7 behind, the middle knuckle 189 6.1 ahead). The handle landed some 5.8 units off KH2's, behind the palm. Offline at Valor's idle, in KH1's right palm: bone 2 at (0.4, -0.4, -1.2), where KH2's own is (3.9, -2.1, 3.5) x 0.903 in its palm; the fingers close toward +z (their middle joints at +1.4). Left the same, mirrored. The turn was right (bone 2's axes within 1.7 degrees of KH2's, t829). Checked (files: `xa_ex_0010.mdls`, `P_EX100.mdlx`, Valor's model and weapon motions, offline).
  - Now: bone 2's place is taken from KH2's middle knuckle (189 right, 97 left) and put on KH1's (113, 217), times the hip ratio; the turn is only the palms' bind turn. Offline: Valor's idle right (-1.9, -1.1, 3.6), left (-0.5, -0.8, -3.4), both on the fingers' side. KH1's own grip (vanilla, every motion) puts 282 at (-0.1, -0.5, 4.0): within about 2 units, toward the wrist. Checked (offline).
  - `kh2slots::slot_key` and `holder_key` add "knuckle anchor": Valor's slot motions and holder motions rebuild once.
  - Assumption still open: KH1's Kingdom Key `.wpn` sits on 282 as KH2's `W_EX010` sits on bone 2 (handle on the bone's x, teeth toward -y); the `.wpn` mesh isn't read yet. Vanilla KH1's grip has 282's y the other way round from KH2's bone 2 in the hand.

### 2026-10-29: Stage 850 (testing branch: the left Keyblade turned the right way in Sora's left hand)
- The owner (chat, with screenshots of KH2 Valor, our Valor idle at trace frame 7071 and vanilla KH1 Sora's grip: "the keyblades still aren't gripping properly in the palm, the left keyblade is pointing in the wrong direction").
  - Cause: KH1's skeleton pass reads an actor's world matrix with its second and third rows flipped (`pose::from_param`: columns x, -y, -z). The holder's world is Sora's left wrist matrix as KH1's attach writes it, so the holder's bones came out turned 180 degrees about the wrist's x. The offline bake puts the left Keyblade's 282 x in the palm frame at (-0.48, 0.88, 0.00); the t840 trace line measured (-0.48, -0.88, 0.00). Checked (trace, offline bake, decomp).
  - `pose::Baker::holder_poses` bakes the holder's bones as `scale(1, -1, -1) × kh1[205]⁻¹ × bone`, so the pass's flip brings them back to the hold `kh2_hold` computed. `kh2slots::holder_key` adds "pass row flip": the slot cache rebuilds the holder motions once.
  - The right Keyblade (Sora's own, in his main bake) has no such flip. Offline its bone 2 sits within about 1.3 degrees of KH2's in the hand frame (wrist to middle knuckle, index to little). Not changed; measuring it in game is still open.

### 2026-10-29: Stage 849 (testing branch: the left Keyblade holder attached to Sora's left wrist)
- The owner (chat: t848 "its fixed now, now that we discovered the problem properly were our other ways of implementing the 2nd keyblade better?"; my answer: the holder design was right, only its bone wasn't; "go ahead").
  - KH1 attaches the holder to Sora's left wrist (205, a drawn bone: bone table `+0x2C` bit 30) instead of his palm (206, a non-drawn one that KH1's blend pose and draw queue don't keep through blends, t848): the row's bone `+0x50` and KH1's attach (`0x1402A06E0`) take `0x2000 | 205`. Checked (files: `xa_ex_0010.mdls`, 205 flagged, 206 not; decomp; the t847 traces).
  - `pose::Baker::holder_poses` bakes the left Keyblade's bones from the wrist (`kh1[205]⁻¹ × bone`) instead of the palm; the grip itself is computed as before (`kh2_hold` on the palm of the same KH1 pose), so the hold is unchanged, the palm's offset from the wrist at each frame baked in. `kh2slots::holder_key` adds "wrist space": the slot cache rebuilds the holder motions once.
  - t846..t848 taken out: the second KH1 draw of Sora with the left Keyblade (`leftblade::left_in_sora`, `holder::left_in_sora`), the re-anchoring onto his palm, the palm-offset memory and the palm read at the holder's step. The holder is drawn by KH1's draw of its own actor again (t833's), and its trail and the left hits read its bones as KH1 posed them. Nothing of ours runs per frame for its place any more (its think keeps its local transform still and asks its motions, as before).
  - The trace line measures from Sora's left wrist now; the t847 bone count is gone with t848's code.

### 2026-10-29: Stage 848 (testing branch: the left Keyblade on Sora's drawn wrist; KH1 keeps only drawn bones through a blend)
- The owner (chat: "I see some differences but the core issue is still there, recorded a trace", t847 traces `trace_1793248994_f926`, `trace_1793249000_f1262`, `trace_1793249015_f2168`). The new count: on every frame the left Keyblade flew, exactly 151 of Sora's 293 bones were over 250 units from his place (bones 0, 1, 3, 4, 6, 7, … among them); on all others 56..57 (his non-drawn bones that sit away from him anyway). The palm stored and the one built from the wrist and the palm's local (t847) were both off by 800..8500 then. Checked (trace).
  - Cause: KH1 Sora's model draws with 142 of its 293 bones, the ones with bone table `+0x2C` bit 30 (Checked (files: `xa_ex_0010.mdls`)); 293 − 142 = 151. The queue (`0x1401D4070`) copies only those (and bone 0) for the draw, and KH1's blend pose (`0x140392560`) works from the same flags, so through a blend the non-drawn bones keep stale world and local matrices with the world translation added again each pass. His body and his own Keyblade (drawn bones 282..291) stay right; the left palm 206 is a non-drawn bone (its wrist 205 a drawn one). Checked (decomp, files, trace). Every reading of the palm since t830 (KH1's attach of the holder, t843..t847) read a bone KH1 doesn't keep through blends.
  - `holder::sora_palm_built`: when the palm sits on the wrist (within 40 units), the palm as it is and its offset from the wrist kept; when it doesn't (a blend), the wrist (a drawn bone, kept right) times the last offset kept. `left_in_sora` moves the left Keyblade onto that palm (t846's draw in Sora's own entry).
  - Where else this can appear (non-drawn bones read during play): KH1's attach of the holder (206; its result is moved by the above), effects bound to a raw bone by `kh2dpx` (`Anchor::Bone`, `0x14029EA00`) when the bone isn't a drawn one, and `pose`'s use of the right palm 101 (a non-drawn bone) on its own drawn routes. Listed, not changed (no request for them).

### 2026-10-29: Stage 847 (testing branch: the left Keyblade put on Sora's palm built from his wrist)
- The owner (chat: "look anywhere in our code where this issue could appear because its still happening", screenshot at KH2's battle idle frame 0 with the left Keyblade lying far off; t846 trace `trace_1793248588_f889`). The trace: the left Keyblade was drawn in Sora's own queue entry every frame, and his palm entry at his draw was 0.0 from the one at the holder's step; yet that entry ran off (the holder's place (-674.6, -662.6) with Sora at (-66.5, -68.3), frame 882) while his body, drawn from the same copy, was right. So the drift is in some of his bones, not his whole array. Checked (trace).
  - Our code that writes Sora's bone array (`+0x1B4`), audited: `pose`'s own drawn pose (all bones, only on its follow routes; the traces are all on the baked route), its blend back into KH1's animation (all bones, only after a pose of its own), `dream_hold_world` (280..292, dream weapons), `leftblade::collapse_weapon` (280..292, KH2's model only), t846's second draw (280..292, put back). None writes the palm (206) alone. Checked (code).
  - KH1's joint words: the palm 206 is a "built once" bone in KH1's own motions (`0x02000000`), its wrist 205 an animated one (0); the left arm's IK bones are 198 (`0x02600000`) and 200 (`0x02804000`), which our bake turns into plain FK. Checked (files: `xa_ex_0010.mset` slots 0, 1, 3, 5, 6; `xa_ex_0010.mdls` for the chain 206 → 205 → … → 0). That the built-once palm's stored world entry is what drifts while its time holds is an Assumption (the trace's count of drifting bones will show).
  - `holder::left_in_sora` takes Sora's palm as KH1's bone pass builds a bone: the wrist's world (`+0x1B4[205]`) times the palm's local (`+0x1B8[206]`), instead of the palm's stored world. The trace line adds how far the stored palm is from the built one and how many of his bones are over 250 units from his place (and the first of them).

### 2026-10-29: Stage 846 (testing branch: the left Keyblade drawn in Sora's own queue entry)
- The owner (chat: "its still happening I recorded the trace, lets go with (b) instead", t845 trace `trace_1793248249_f4119`). The trace: on all 301 lines no palm had been kept, the holder was queued before Sora's own draw every frame, so t845 moved nothing. Checked (trace).
  - (b), the way KH1 draws its own Keyblade: KH1's actor draw (`0x1402A1630`) puts an actor's weapon model in its queue entry and the queue (`0x1401D4070`) copies the instance's whole bone array at that moment, body and weapon (its last bones, 280..292) from one copy. Checked (decomp).
  - `leftblade::actor_draw`: right after KH1's draw of Sora (his field instance, Valor on KH1's states), `left_in_sora` runs KH1's same draw on Sora again with the hidden-mesh mask 0xFFFFFFFF and, for that call only, the left Keyblade's KH1 weapon object in his `+0x14C`, his flag 0x4000, and the left Keyblade's 13 matrices in his weapon bones; then his object, flags and bones are put back. KH1's queue copies them from his array at the same moment as his body. No shadow comes of it (the shadow pass is the caller's, `0x1402A1B70`).
  - The matrices (`holder::left_in_sora`): the holder's weapon bones as KH1 posed them from the holder motion, moved rigidly from the palm KH1 attached the holder to in its step (`palm_at_step`) onto Sora's palm in his array at that draw. Written back into the holder's bones too (its trail, the left hits read them after). The holder itself isn't drawn any more (`holder::draws_in_sora`): KH1 still makes, steps, poses and blends it, and runs its trail and hits.
  - t845's `before_draw` is gone (it never acted). The trace line says whether the left Keyblade was drawn in Sora's entry and how far his palm at his draw was from the one at the holder's step.
  - Open: the trail and the left hits read the holder's bones before the draw too (KH1's binding task, the attack builder), so on a drifting frame they may still read the drifted palm.

### 2026-10-29: Stage 845 (testing branch: the left Keyblade re-anchored on Sora's palm as KH1 queues his draw)
- The owner (chat: t844 "its still happening, please investigate why kh1's own keyblade during animations doesn't fly off and tell me how that could be applied to the second copy"; answer (a): "do (a) for now").
  - Why KH1's own Keyblade stays in his hand: KH1's actor draw (`0x1402A1630`) puts the weapon's model into Sora's own queue entry, and the queue (`0x1401D4070`) copies the instance's whole bone array (`+0x1B4`, 293 matrices) at that moment; the body is drawn from the copy and the weapon model on its last bones (body count minus the weapon's: 280..292). Body and blade are one copy. Checked (decomp).
  - The holder is its own actor: its place built from his palm in its step (6000), posed at 8000, copied when KH1 queues it. The t839..t844 traces: the palm it hung on ran off while he blended (his time held), his body drawn right. What changes his palm entry between the holder's step and his draw is not found (open).
  - `holder::before_draw` (from `leftblade`'s actor-draw hook, before KH1's draw of an actor): Sora queued, his palm (`+0x1B4[206]`) kept as KH1 copies it for him; the holder queued later that frame, its 13 weapon bones moved rigidly from the palm it was attached with in its step (`palm_at_step`, its think, before KH1's attach build) onto the kept palm, and its `+0x50` onto that palm. With no palm kept this frame (holder queued first) nothing moves. KH1 still makes, poses, blends and draws it.
  - t843's write of `+0x50` / the instance's `+0xF0` and t844's local offset (`palm_as_drawn`) are taken out: both rested on guesses about which palm was right; the holder attaches with no offset again (`hold_still`).
  - The trace line says how far the kept palm was from the one the holder was attached with, or that none was kept.

### 2026-10-29: Stage 844 (testing branch: the left Keyblade holder put on Sora's palm as drawn while his bones drift in a blend)
- The owner (chat: "recorded the trace go ahead with the fix", t843 trace `trace_1793247279_f1436`). t843's fix changed nothing: in every line KH1's `+0x50` before it already equalled the holder's place `+0x100`, and that place is 0.0 from Sora's left palm in his own bone array (`+0x1B4[206]`). So the palm entry itself runs off: over his walk / run blends (his clip frame held at 0.0 / 4.0) it reached about nine times his place (frame 1276: (-1394.9, -4642.1) with Sora at (-138.3, -466.2)), back on him when his time moves. His model is drawn right all the while. Checked (trace).
  - KH1's blend pose (`0x140392560`) builds the bones and adds the world translation to every bone matrix in `+0x1B4` each pass; `pose`'s placement note already has KH1's blends throwing the placement taken from his bones (its pass-matrix rule). KH1's draw takes his model's place from the actor (`+0xD0` turns, `+0x50`, `0x1402A1630`). The holder's attach (`0x1402A0A40` × `0x14029EA00`) reads the drifting palm entry. Checked (decomp). That the drift is one translation for every bone is an Assumption (the pass adds one to all).
  - `holder::palm_as_drawn`, in its think (before KH1's attach build at the end of the same step): the palm taken into his model space through the frame his bones carry (bone 0 × `+0x1B8[0]`⁻¹, which drifts with them) and out through his drawn frame (`+0xD0`'s turns with the pass's row flips, as `pose::from_param`, at `+0x50`); the holder's local offset `+0x10` set so KH1's own local × palm lands there. Under a unit (the two frames agree: no blend) it stays 0. KH1 still builds, poses and draws it; only the offset it attaches with changes.
  - t843's `before_draw` stays (it puts `+0x50` on the place, now the drawn palm). The trace line adds how far the palm was moved.

### 2026-10-29: Stage 843 (testing branch: the left Keyblade holder drawn at its place)
- The owner (chat: "the keyblade is still leaving soras hand", t842 trace `trace_1793246939_f2145`, screenshot at KH2's battle walk frame 0). With t841's no-travel holder motions the holder's `+0x50` still runs off: right after each holder motion asked, it grows by about one palm place a frame ((-280, -924), (-427, -1377) ... (-1456, -4548) with Sora at (-147, -453), the palm's height kept) for as long as Sora's own blend holds his time (his clip frame 0.0 or 4.0, 8 to 10 frames), and is back on the palm the frame the time moves on. The holder's bones, grip and place `+0x100` stay on the palm throughout, its blend 0. Checked (trace).
  - KH1's pose pass sets the instance's root `+0xF0` to the world translation plus the travel helper's (`0x140391C70`), and the pose step's no-blend branch copies it into the actor's `+0x50` each frame (`0x1403948D0` from `0x14029F0A0`); Sora's own `+0x50` comes from the blend branch while he blends. Checked (decomp). Where the per-frame growth comes from inside the pass while the time holds is not found yet (Assumption: a helper or scratch value carried over between passes at the same time).
  - `holder::before_draw`, from the actor-draw hook `leftblade` already has (`0x1402A1630`), right before KH1 draws the holder: `+0x50` and the instance's `+0xF0` set to its place `+0x100` (the palm), the one place KH1 keeps for an actor on its own (`pose`'s "drawn at" check: `+0x10`, `+0x50`, `+0x100`). Nothing else changes; KH1's next pose writes them again. The trace line adds the value KH1 had there before.
  - t841's no-travel holder motions stay (KH2's left weapon has no travel of its own).

### 2026-10-29: Stage 842 (testing branch: no shadow off the left Keyblade holder)
- The owner (chat, screenshot at trace frame 2720: "you can also now see a copy of soras shadow t-posing"). KH1's per-actor draw (`0x1402A1B70`) runs its shadow pass (`0x1402BD6F0`, placed at `+0x50`, from the model's bones) for an actor with `+0` bit 0x2; the holder kept the bit from Sora's row, and its body is Sora's model at the bind with every mesh hidden, so its shadow was a T-posed Sora. Checked (decomp; screenshot).
  - `holder` clears `+0` bit 0x2 at its make and in its look each frame. KH2's left weapon casts no body shadow; Sora's own is his actor's.

### 2026-10-29: Stage 841 (testing branch: the left Keyblade holder motions baked with no root travel)
- The owner (chat: the Keyblade "moving so fast away from sora but is still being rendered", answer (b), and a screenshot of the blade lying off on the floor at KH2's battle run frame 0; t840 trace `trace_1793246069_f985`). The trace: the holder's bones stay on Sora's palm (grip 1.9 to 4.4 units, place `+0x100` 0.0), drawn every frame with its weapon, its model and weapon-out set; but its `+0x50` goes thousands of units out after each walk / run change (frame 728: (719.7, 8190.1) with Sora at (74.0, 820.0); idle frame 686: (8.3, -58.0, 894.8) with Sora at (33.1, 897.9), his left palm's height). Checked (trace).
  - Cause: KH1's pose step copies the instance's root travel (`+0xF0`, written by the pose pass, `0x140391C70`) into the actor's `+0x50` (`0x1403948D0` from `0x14029F0A0`), and KH1's draw builds the model's matrix from `+0xD0`'s turn and `+0x50` (`0x1402A1630`), as the shadow pass uses `+0x50`. The holder motions were baked on the KH1 slot's own template with no travel given, which keeps the template's travel helper (H10) translation channels (`kh1_data::bake`): the walk / run holder motions carried KH1's walk / run travel. Checked (decomp; `docs/kh1-weapon-second.md` §7, `kh1_data::bake`'s rules; trace).
  - `kh2slots::holder_one` bakes with a zero travel per frame (the template's H10 translation channels dropped, none written; a template with no helpers is baked as before); `holder_key` adds "no travel", so the slot cache rebuilds them. KH2's left weapon has no travel of its own: it rides the palm, and the palm carries Sora's.
  - Not checked yet: the template H10 record's own resting translation (frame 0's) on the walk / run templates; if it isn't 0 it stays as a fixed offset. The t839 / t840 trace lines stay until the next trace shows `+0x50` on the palm.

### 2026-10-29: Stage 840 (testing branch: the holder measure adds the blade's turn and KH1's weapon-draw tests)
- The owner (chat: "recorded the trace", t839 traces `trace_1793245784_f3946`, `_f4396`, `_f729`, `_f869`). Every row but one (f842) has the holder drawn, and through every idle / run / stop change (KH1 slots 242 / 245 / 248, holder motion asked each time) its place is on Sora's drawn left palm (0.0), its grip (280) and blade base (282) 1.1 to 4.4 units from it, its blend 0. Checked (trace). So the blade's place doesn't leave the palm in KH1's bones as drawn; what the owner sees is then either a turn of the blade (not measured) or a frame where KH1's draw passes it over (KH1 draws a weapon only with `+0x14C`, its model and `+0` bit 0x4000, `0x1402A1630`, Checked (decomp)). (The model root at 92.7 is the holder's bind root, unused: the weapon bones are placed from the palm.)
  - `holder::note_drawn` adds: bone 282's turn in the palm's frame (the largest axis change since the last draw, in degrees, and its x axis there), and the weapon / its model / weapon-out / drawn bits, alpha, hidden-mesh mask and `+0x374`. Trace only, as t839.

### 2026-10-29: Stage 839 (testing branch: a trace measure of where KH1 draws the left Keyblade holder)
- The owner (chat: "my keyblade still leaves soras hand and appears, you can see when I transition from idle to running", t838 trace `trace_1793245021_f3154`). The trace has no column for the holder, so it can't show where the blade went; the t838 log shows the room change now frees and remakes the holder cleanly (f421). Checked (log, trace). What I read in the decomp: KH1's attach build (`0x1402A0A40`) makes the holder's world its local one times Sora's palm as `0x14029EA00` reads it (a raw bone: Sora's instance's bone matrix, `0x1401D6220`), so the place itself follows his posed palm. Checked (decomp). Which step leaves the blade off the hand is not known yet.
  - `holder::note_drawn`, called from the actor-draw hook `leftblade` already has (`0x1402A1630`) when KH1 draws the holder and the trace is recording: one events line a frame with the distances from Sora's drawn left palm (bone 206's `+0x1B4` matrix) to the holder's place (`+0x100`), its model's root (bone 0), grip (280) and blade base (282), and its slot, time, blend length / time, queued requests and `+0x50`. `holder::sync` adds a line at each holder motion asked. Nothing runs while the trace is off. (The draw comes after Sora's row is made in a frame, so its line lands in the next row.)

### 2026-10-29: Stage 838 (testing branch: the holder's new model not taken for Sora's while KH1 makes it)
- The owner (chat, same t835 log). At the holder's make (f125) the log read "Sora's instance 0x7ff72dc72dd0" before "left Keyblade holder: made", then "Sora's actor at" the holder, "retarget ready" and the KH2 set-ups twice, and Sora's own instance back a frame later (f126). KH1's maker poses the new actor's model inside the call, before `holder` knows its actor, and `pose`'s Sora check takes any 293-bone model within 150 units of his actor for his (the holder is a copy of his model on his palm): for that frame KH2's passes ran on the holder and Sora's retarget was rebuilt twice. Checked (log).
  - `holder::making()` is set around KH1's maker call; `leftblade::is_holder` counts any model posed then as the holder's, so `pose`'s hooks hand it straight to KH1's pass. (The maker runs from `holder::frame`, after Sora's own pass, so nothing else is posed inside it. Checked (code).)

### 2026-10-29: Stage 837 (testing branch: the left Keyblade holder's weapon taken off before KH1 frees it)
- The owner (chat: "when I left the room the game crashed", t835 log: `CRASH 0xc0000005 at exe+286992 (reading 0x0)`, `rsi` the holder actor, `rcx` 0xFF, from `exe+292eae`). KH1's "actors freed" task (`0x140292E40`, priority 163001, every frame) frees each actor marked dying (`+0x374` bit 2, not bit 0x800000): its model instance, then, with a weapon object at `+0x14C`, its weapon's give-back `0x1402868E0`. That finds the weapon's load slot by the actor row's member `+0x4C` among KH1's three (`DAT_14296B540`) and writes through the found slot; the holder's row has member 0xFF (t830, so no party data), no slot matched, and it read through null. An area change marks every actor dying, so the holder still carried the left Keyblade's object of ours into it. Checked (decomp; log registers).
  - `holder` hooks the freed task (first bytes `48 89 5C 24 10` checked; without the hook the holder isn't installed): when the holder is dying and not kept, its `+0x14C` and weapon-out bit are cleared and it is forgotten, then KH1's own pass runs and frees it. One load and compare a frame otherwise. Our own end (`holder::end`) already cleared the weapon before KH1's actor end, so only KH1's own teardowns needed this.
  - The module doc's "an area change drops every actor without telling" was wrong: corrected (marked dying, then freed by `0x140292E40`).

### 2026-10-29: Stage 836 (testing branch: no KH1 blend on the left Keyblade holder)
- The owner (chat: "the keyblade appears but it disappears inbetween transitioning between states", t835 log). The holder asked each new holder motion with Sora's blend length (8 to 10 frames at his combo and run changes, log). KH1's blend start (`0x1402A0350`) keeps the model root's offset from the actor (`+0x270`, and `+0x220` through `+0x230`) and its blend step (`0x14029F0A0` → `0x1402A0620`) moves the actor's place and drawn matrix by it until the blend ends: root-motion handling for an actor standing on its own. Checked (decomp). That this is what threw the blade off the palm during those frames is an Assumption (no trace of the holder): it fits the blade being gone only between states.
  - `holder::sync` asks with no blend (0.0): the start then takes KH1's no-blend path (`+0x160` bit 2 test, `0x1402A0350`). KH2's left weapon sits on its palm bone with no blend of its own; Sora's own blend still moves the palm, so the hand's change stays smooth and only the palm-relative grip changes at once; how big that change is between two holder motions isn't measured yet.

### 2026-10-29: Stage 835 (testing branch: the holder's first-bytes check reads past a short hook of ours)
- The owner (chat: "still doesn't appear", t834 log). The holder still logged "not installed (KH1's motion request ... isn't the Steam build's)": the probe's set-motion hook moves only 5 bytes (`48 89 5C 24 08`, `probe.rs` `SETM_PROLOGUE`) while the holder checks 10, and t834's `hook::prologue_is` wanted the moved copy to cover all 10. Checked (log, `probe.rs`).
  - `prologue_is` now takes the moved bytes for the part a hook of ours covers and the live bytes after it (still KH1's own: the hook writes only its `jmp rel32` over the bytes it moved).

### 2026-10-29: Stage 834 (testing branch: the left Keyblade holder installs past our own set-motion probe)
- The owner (chat: "the 2nd keyblade doesn't appear", t833 log and trace). The log's line "left Keyblade holder: not installed (KH1's motion request at ... isn't the Steam build's)" came right after our probe's "hooked set motion" at the same address: the probe (`probe.rs`, Epic `0x29D7F0` = Steam `0x14029F980`) had already put its jump over KH1's motion request, so the holder's first-bytes check saw our jump and refused, and with the holder off nothing drew the left Keyblade. Checked (trace log; exe bytes at `0x14029F980` are `48 89 5C 24 08 57 48 83 EC 20`, as the holder expects).
  - `hook::install` now records each hooked function's original first bytes; `hook::prologue_is(target, bytes)` accepts a function whose live bytes match or that one of our hooks moved those exact bytes out of. The holder's six checks use it (it only calls these functions, so going through the probe's detour is fine: the probe notes the request and runs KH1's).
  - The other five functions the holder calls (maker `0x140290D60`, task maker `0x14028A900`, step `0x140297FE0`, end `0x140292390`, attach `0x1402A06E0`) are hooked by nobody in that log. Checked (log).

### 2026-10-29: Stage 833 (testing branch: the left hits and second-weapon effects on the holder actor; the copy off for Valor)
- The owner (chat, the native thread's second-Keyblade plan; answers 2 (a): no left Keyblade while KH1's pool has no place, 3 (b): the old path deleted). Last of t830 to t833.
  - `leftblade::left_owner` / `left_owner_handle` give the holder actor (and its KH1 handle) on Valor's KH1 states, once it carries the left Keyblade's object and has been posed: `lefthits`' attack object takes it as its bone-source actor (`+0x90`), so KH1's builder (`0x1402B3800`, bone point `0x14029EB20`) lays the blade's spheres along the holder's own posed blade with the left Keyblade's weapon row; `kh2dpx`'s second-weapon points (0x4004 / 0x4006 / 0x4008) bind to it, so KH1's binding task reads its bones every frame. Checked (decomp, as t680 / t697).
  - On Valor's KH1 states nothing of the copy runs: `fill_owner`, the second actor-draw call and our trail-step call are off there, and with no holder actor (KH1's pool full) nothing of the left Keyblade shows until it is made. The copy now serves Final Form only (KH2's controller, left as it is).
- Assumption: KH1's attack pass for the left object runs after the holder's pose (8000) in a frame; if it runs before, the left hits use the blade as posed the frame before. Not checked: in game. `docs/kh1-weapon-second.md` §12.

### 2026-10-29: Stage 832 (testing branch: the left Keyblade drawn and trailed by KH1 on its holder actor)
- The owner (chat, the native thread's second-Keyblade plan). Third of t830 to t833. Checked (decomp + exe, Steam):
  - KH1 draws every actor by its own draw task (`0x1402A0990` makes it: type 6 → `0x1402A1B70`), which needs `+0x374` bit 4 (stepped this frame), a model, actor `+0` bit 0 and alpha over 0, culls by `0x1402A1200` unless `+0x374` bit 0x20000, and calls the actor draw `0x1402A1630 (actor, +0x38C mask, +0x394, colour)` with the colour from `+0x370` (fading to `+0x350..`), `+0xA0..`, `+0x60` and `+0x430`; the actor draw queues the weapon object `+0x14C` while actor `+0` has 0x4000.
  - The holder's think (`holder::look`): `+0x14C` = the left Keyblade's KH1 weapon object (`leftblade::left_object_now`; none while KH2's model draws it) with 0x4000, Sora's drawn bit, colours and alpha (its fade rate `+0x64` kept 0: KH1 ends an actor that fades out), no frustum cull. KH1's draw task then draws it on the holder's weapon bones; `leftblade::left_draw`'s second actor-draw call is off while the holder is out.
  - The trail: KH1's actor step runs the trail step `0x1402C1F20` on any actor with a weapon object; the pose step sets `+0x1CC` to the instance's level events (`0x14029F0A0` → `0x1403948B0`). The holder motions now carry the left Keyblade's trail windows (the move's 7.1) as KH1's own 7.0 (`kh2slots::holder_one`, in the holder key), so KH1 sets the holder's own trail bit; KH1's trail request for Sora is asked for the holder actor (`leftblade::trail_start`, `trail_holder`); our extra trail-step call is off while it is out. Always (Cosmetics) sets its bit and asks its trail in its think. Its trail is ended before the actor ends (`leftblade::end_left_trail`).
- Still `leftblade`'s copy: the left hits and KH2's second-weapon effect points (t833). Final Form unchanged. Assumptions: copying those colour fields matches every fade KH1 gives Sora; the trail's owner may be any actor with a weapon object. Not checked: in game. `docs/kh1-weapon-second.md` §12.

### 2026-10-29: Stage 831 (testing branch: the left Keyblade's holder posed and blended by KH1 on its actor)
- The owner (chat, the native thread's second-Keyblade plan; answer 1 (a): every slot Valor plays gets a hold, "look at which bones attach the keyblade"). Second of t830 to t833. Checked (decomp + exe, Steam):
  - An actor's motion table `+0x1D4` is a handle to an array of motion handles by slot, ended by 0 (`0x1401D6240`); KH1's motion request `0x14029F980 (actor, code, blend)` (blend in XMM2, `movss [rcx+0x184]`) queues one (`+0x178` count, `+0x17C` code, `+0x180` start time, `+0x184` blend), bit 0x20000000 a raw slot past the `+0x1D8` remap; the pose step `0x14029F0A0` starts it with `0x1402A0350` (KH1's blend when one plays and the length is over a step), then advances `+0x16C` by dt × `+0x284`.
  - `holder::sync` (its think): the holder motion of Sora's motion (`ptr(+0x1D4)[+0x168]`, `kh2slots::holder_motion`), or on a slot of KH1's own Valor's idle holder at frame 0 (`kh2slots::rest_holder`: KH2 keeps the left weapon on its palm); a new one goes into the other of two entries of our table and is asked of KH1's request with Sora's blend length (`+0x170`) at his time; else its time is Sora's. Its own speed is 0. KH1's pose task (8000, after Sora's at 5000) poses and blends it.
  - The holder motions are rebaked in the left palm's own space (`Baker::holder_poses`: the weapon bones × the palm⁻¹), the actor's root being the palm (t830), so KH1 leaves them on Sora's palm as drawn. `leftblade::posed_left` reads its instance's world matrices (`+0x1B4`, 280..292).
  - Gone: our pose instance and `pose_holder`, the hooks on KH1's two blend starts and their heap checks (`blend_holder`, `heap_ok`, `end_blend`), the hand carry and `LAST_HELD` (KH1's own slots now have Valor's idle hold), the per-frame KH2 left weapon on baked slots while the holder is out, `kh2slots::has_holder`, ini `left_blade_blend`.
- Still `leftblade`'s copy: the draw and trail (t832) and hits (t833), which read `posed_left` (the hits and trail during Sora's step, so the frame before). Final Form unchanged. Assumptions: Sora's `+0x170` is the length of his blend running (0 when none); writing the holder's `+0x16C` each frame matches what KH1's own step would reach. Not checked: in game. `docs/kh1-weapon-second.md` §12.

### 2026-10-29: Stage 830 (testing branch: the second Keyblade's holder as one of KH1's own actors)
- The owner (chat, the native thread's "second Keyblade's actor copy", go ahead; answers 2 (a) and 3 (b)). First of four versions (t830 to t833) moving `leftblade`'s copy of Sora's actor onto a real KH1 actor. New `win/holder.rs`. Checked (decomp + exe, Steam):
  - KH1's actor maker `0x140290D60 (object id)` takes a row of an object table (`0x140287B40`: 0x78-byte rows; an id whose byte 2 isn't 1, 7, 8 or 0xC is matched whole in the area's general table `DAT_14296B630` / count `DAT_14296B628`, rows inside the loaded area file). Our row is a copy of Sora's (`+0x130`: his model handle, motion set and names, so his loaded model is shared) with id `0x00064B48` (type 6: tested nowhere, full-model path of `0x140288460`), flags +0x1 / +0x2 (no asset load, no automatic spawn) and without 0x100 / 0x20 / 0x200 / 0x10000, member 0xFF, class 0xFFFF (no party data, no weapon object, `0x140286D60`), kind 6 (no think; parent from the row `+0x52` names, `0x140287960`), cost / placement / group 0, sizes and mass 0. The table points at a copy with the row added for the maker's call only, then back.
  - Attached by KH1's own attach `0x1402A06E0 (actor, parent, 0x2000 | 206)` to Sora's left palm: KH1's world step `0x1402A0A40` makes its world its local one times the palm's drawn matrix; `+0x374` bit 0x1000 with no turn, no offset and scale 1 makes that the palm. An actor with a parent skips gravity and the map (`0x140292010`).
  - Its step task moved to 6000: KH1 steps actors from one list by priority (Sora 2000, kind 6 at 4000, pose tasks at -1000 / 5000 / 8000, each posing the actors stepped since the last, `0x140298730`); at 4000 it would read the palm a frame late. KH1's task maker `0x14028A900` makes it again at 6000 (KH1's kinds 10 and 13 step there) with KH1's actor step `0x140297FE0`, the actor and our think, as `0x140297980` makes one; the maker's own task gets a function returning 4, which the list runner `0x14028A710` unlinks and frees. Posed at 8000, after Sora at 5000.
  - Made when Valor's left Keyblade is out on KH1's own states and none is alive (`holder::frame`, after Sora's pose pass), ended with KH1's actor end `0x140292390` when it is put away (also from its think); no actor while the pool's 96 are taken (the maker would write through a null pointer), tried again a second later, logged. An area change drops every actor (`0x140291F40`); it is made again.
  - Hit set `+0x134` and team bits (`+0x374` 0x60) cleared (no hit or lock-on target, out of the team lists `0x140291A50`), hidden-mesh mask `+0x38C` all, no weapon object, its own motion's speed 0: nothing of it shows yet. `leftblade::is_holder` also takes its instance (KH1's passes on it go straight to KH1's).
- Still `leftblade`'s copy: the pose, draw, trail and hits (t831 to t833). Assumptions: no shadow or other per-actor list picks up a type-6 actor (not found in the decomp); the first frame of Sora's own first motion, which the maker starts on it, has no motion events. Not checked: in game. `docs/kh1-weapon-second.md` §12.

### 2026-10-29: Stage 829 (testing branch: Valor's two Keyblades held as KH2 holds them)
- The owner (chat, on the native thread's second-Keyblade plan, answer 1): "look at which bones attach the keyblade to soras model because currently valor is really off model on both keyblades", then "(b)": Valor only for now, base Sora later. Checked (files): KH2 hangs a weapon on a palm bone, `sklt` row 1 (P_EX100 and every form's objentry weapon joint): 178 right, 86 left, each the parent of its five fingers; KH1's `.wpn` is skinned to 280..292, 280 a child of the right palm 101 (left palm 206); the palms' binds agree within 3 degrees. KH2's Keyblade skeleton (`W_EX010.mdlx`) is KH1's 280..291 bone for bone (bind turns within 0.3 degrees, the same keychain lengths), and its mesh is skinned to bone 2 (834 corners, blade along -x), the keychain to 3..11. KH2's weapon motions put a half turn about the x = y diagonal in bone 1 where KH1's bind has a half turn about y in 282, so only bone 2 means the same in both. The bake held KH1's rest grip on 280 with the teeth-up turn and `form_grip`'s change of KH2's root; an offline run of that bake on the owner's files put KH1's 282 about 21 degrees along the blade and 24 round it off KH2's bone 2 in every Valor move, both hands (the left took the right's correction, `leftblade::left_bones`).
- `pose::kh2_hold`: KH2's weapon bone 2 carried from KH2's palm onto KH1's (`kh1_bind[palm]⁻¹ × kh2_bind[palm]`, offsets in KH1's units by the hip ratio), 280 / 281 put under it as KH1's bind holds them (KH1's mesh as KH1 skins it), the keychain carried with it. Valor's main bake uses it for the right Keyblade (178 → 101) in place of the teeth-up turn and `form_grip` (`Baker::poses_of`), and its holder motions for the left (86 → 206, `Baker::holder_poses`) in place of `left_bones`. Offline: within 1.7 / 2.8 degrees of KH2's bone 2 in every Valor move, both hands (the palms' bind difference). Base Sora, Wisdom and Final unchanged (`left_bones` / `note_pose` still serve Final). Valor's slot and holder keys gain "kh2 hold"; every slot rebakes once anyway (new version).
- Assumption: KH1's Kingdom Key `.wpn` mesh sits on its bind like KH2's `W_EX010` (the two binds are the same); not read from the `.wpn`. Not checked: in game. `docs/kh1-weapon-second.md` §11.

### 2026-10-29: Stage 828 (testing branch: KH2 Cure's heal scaled by Sora's max MP)
- The owner (chat): "for the heal amount tie it to the amount of max mp you have, in kh1 thats how magic is scaled (assumption)". KH2's heal (kind 6) is power × the caster's Magic / 100; KH1 has no Magic stat, so Cure's heal (`formmagic::heal`, the row's power over `CURE_SHARE` of the max HP) is now times `combat::magic_factor` (Sora's max MP: +10% a point, at most 30, times the ini's `magic_scale`), the stand-in the mod's other Magic-based hits use. KH1 scales its magic by max MP: Checked (decomp + files) afterwards, see `docs/kh1-magic-native.md` §8b (KH1's own Cure heals its data's power 15 / 27 / 36 + max MP, stat source 1). Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 827 (testing branch: KH2 spell scripts: Wisdom Form's shot script on a KH1 process)
- The owner (chat): "convert the rest thats still open over, don't stop until your done, if kh2 cure is on then it should behave like kh2's own cure". `Magic::object_effect` (Wisdom's shots, the form model's script run by its motions' frame trigger 1) starts on the host's machine as a cast does (`NativeHost::machine` with trigger 0x10): a KH1 script process, its first run at once as before, a thread it starts run in that same step (as KH1's machine runs a child started in its parent's step). Offline test `wisdom_shot_on_the_host_as_our_arena` (the tester's own Wisdom files): the same shot and effects, frame for frame. Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 826 (testing branch: KH2 spell scripts: effect ends, fades and moves by their calls)
- The owner (chat): "convert the rest thats still open over, don't stop until your done, if kh2 cure is on then it should behave like kh2's own cure". `trap_effect_loop_end` ends the effect's KH1 instance at the call (`World::effect_end`, `Effect::host_ended`), `trap_effect_fadeout` hands the fade to our effect tick on KH1's instance alpha (`kh2dpx::spell_fade`, `Effect::host_fading`), `trap_effect_set_pos` / `set_dir` place an unbound one at the call (`World::effect_place`). Our step no longer ends, fades or places those; what no call does (an owner gone, a motion change, a cast's end, Reflect's hold and combo wait) stays with it. Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 825 (testing branch: KH2 spell scripts: Reflect's barrier placed by its calls)
- The owner (chat): "convert the rest thats still open over, don't stop until your done, if kh2 cure is on then it should behave like kh2's own cure". The barrier's KH1 guard object (t796) is made and moved by the script's `trap_attack_set_pos` (Checked (files): REFLECT's script places it every frame it stands; offline trace) through `KhWorld::attack_set` (`Strikes::barrier_place`, KH2's hit state taken with `takeover::with_strikes`), not while the combo hides it, and freed by `trap_attack_free` (`combat::barrier_free`). Our step only takes where attacks met it (`Strikes::barrier_met`) and places it itself when no call did since the last frame (a cast its script machine runs inside the step). Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 824 (testing branch: KH2 spell scripts: Blizzard's push under KH2's controller)
- The owner (chat): "convert the rest thats still open over, don't stop until your done, if kh2 cure is on then it should behave like kh2's own cure". Blizzard's push (`hitreact::script_push`, t821) starts KH1's push on Sora under KH2's controller too (Wisdom Form); KH1's physics step, which moves him by it (`0x1402C7070`, Checked (decomp)), doesn't run then, so the controller's move takes it as that step would, each 60 Hz step on KH2's curve (`hitreact::controller_push`, added to the move with the carry in `takeover`). Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 823 (testing branch: KH2 spell scripts: ground and walls from KH1's rays)
- The owner (chat): "convert the rest thats still open over, don't stop until your done, if kh2 cure is on then it should behave like kh2's own cure". `trap_bg_ground_pos` asks KH1's floor ray down from the point (`0x1401910E0`, as `feet` uses it; until t822 Sora's own floor height). `trap_attack_is_hit_bg` (Blizzard's, Thunder's and Magnet's scripts) is the attack's last move met by KH1's background ray, the map's walls (`combat::wall_between`, `0x140191520`, the one KH1's attacks use; `World::attack_hit_bg`). Assumption: KH2 counts a wall only (not a floor) for its shards; KH2's own bg test on attacks wasn't traced. Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 822 (testing branch: KH2 spell scripts: Donald and Goofy in the scripts' world, KH2's Cure on them)
- The owner (chat): "convert the rest thats still open over, don't stop until your done, if kh2 cure is on then it should behave like kh2's own cure". KH1's party slots 1 and 2 are objects of the scripts' world (`reflect::PARTY0`); `trap_sysobj_party` gives them. KH2's Cure script does its own circle test (Checked (files): party slots 0..3, alive, not a summon, flat distance under its magi record's radius; ATKP 708 on Sora, 709 on the others, kind 6) and strikes each one in it; each strike heals that actor through KH1's own HP add (`0x1402A4920`, Checked (decomp), first bytes checked; KH1's save point heals all three through the same stats fields). Offline test `cure_heals_the_party_in_its_circle`. The heal's size stays ours (`CURE_SHARE`, KH1 has no Magic stat). Not checked: in game. `docs/kh1-magic-native.md` §8b.

### 2026-10-29: Stage 821 (testing branch: KH2 spell scripts: Blizzard's push on KH1's push)
- The owner (chat): moving the scripts' calls onto KH1's own objects, "all (a)" (question 3: Blizzard's caster push through KH1's push), "go ahead". `trap_obj_hitback` on Sora (Blizzard's push of its caster, 50) starts KH1's own push (`0x1402C7300`, first bytes checked before the call; KH1's physics step moves him by it through his collision, `0x1402C7070`, Checked (decomp)) with KH2's speed and decay on it (`0x1403D4960` / `0x1403D4570`, Checked (decomp): distance × 0.1, × 0.9 a frame), through `hitreact`'s KH2 push (`hitreact::script_push`, whatever the Hit Reactions style). Only on KH1's states; not under KH2's controller (logged).
- Assumption: KH2's player bonus to the distance (player record `+0x1C8`) is 0 for Sora. Not checked: in game. `docs/kh1-magic-native.md` §8a.

### 2026-10-29: Stage 820 (testing branch: KH2 spell scripts: effects started and killed by their calls; Reflect into Reflect)
- The owner (chat): "all (a)", "go ahead". An effect a script starts is started on KH1's effect system by the call itself, with its sound (`World::effect_start`, `reflect::Sink::effect_start`; `Effect::host` its instance), and one it kills is stopped by the call (`World::effect_kill`). Ends, fades, moves and Reflect's holds still follow from our step.
- **Reflect into Reflect:** the second cast's release ends the first barrier's element through the shared teamwork block, with a handle of the first cast's; the call looked only in the second cast's own effects, so the first barrier stayed. A handle no cast of its own has is now looked up in the other casts' (on KH1's memory only). Offline test `reflect_chain_on_the_host_ends_the_first_barrier` (fails before, passes after). Not checked: in game. `docs/kh1-magic-native.md` §8a.

### 2026-10-29: Stage 819 (testing branch: KH2 spell scripts: attacks made and moved by their calls)
- The owner (chat): "all (a)", "go ahead". `trap_attack_set_radius` / `trap_attack_set_pos` make or move the attack's KH1 attack object at once (`bodyhits::spell_place`, KH1's constructor), `trap_attack_free` frees it, `trap_attack_is_hit` reads whether KH1's pass hit something with it (`bodyhits::spell_hit`). One not moved stays where it was put with no sweep; one never freed goes inert after 300 frames standing. Our step no longer builds the attack list each frame (`spell_attacks` gone); it takes its runtime's hits only (`spell_results`, by key bit). Reflect's barrier stays the guard our step places (t796). Not checked: in game. `docs/kh1-magic-native.md` §8a.

### 2026-10-29: Stage 818 (testing branch: KH2 spell scripts: bones on KH1's skeleton)
- The owner (chat): "all (a)" (question 1: map the bones to KH1's skeleton), "go ahead". The spells' bone codes (Checked (files)): 0x4000 the actor, 0x4001 KH1's body centre (point 0x8003, `0x14029EB20`, Checked (decomp)), 0x4003 / 0x4004 KH1's Keyblade tip (point along bone 0x30 at 1), 0x4009 the target point, KH2's bone 0 KH1's bone 0 (both models' hips, `Retarget::hip_ratio`) as KH1 drew it (instance `+0x1B4`). KH2's drawn pose no longer read (`pose::kh2_pose` unused). Not checked: in game. `docs/kh1-magic-native.md` §8a.

### 2026-10-29: Stage 817 (testing branch: KH2 spell scripts: the world read from KH1's actors at each call)
- The owner (chat): "The calls still act on our own spell world ... lets transition this as well", then "all (a)" (question 2: KH2 ability 114 mapped to KH1's ability list), "go ahead". `reflect::KhWorld` holds KH1 actors and reads them when a script calls (KH1's script machine runs after the actors and the collision pass): position, facing `+0x3C`, air states (Sora's from our step while KH2's controller has him: KH1's `+0x70` isn't kept then), KH1's alive test (`0x140291FF0`), KH1's lock and lock point, KH1's body centre as Sora's target point, KH1's party table, Sora's floor, KH1's frame time. KH2 Leaf Bracer (114, Checked (files)) is KH1's 62 by KH1's ability count (`0x140284BE0 (62, member)`, Checked (decomp)). `trap_obj_star` (Cure with Leaf Bracer on himself: KH2's no-hit timer, Checked (decomp) KH2 Steam) is KH1's untouchable flag for its frames (`reflect::star`). The kept copy and `refresh` are gone. Not checked: in game. `docs/kh1-magic-native.md` §8a.

### 2026-10-29: Stage 816 (testing branch: Item Pickup: the box shows the pickup's own icon)
- The owner (chat): "when you pick up any world items the potion icon appears instead of the general item icon that was in vanilla, can you change it". `queue_obtained` gave every pickup line the treasure texts' item icon 0x82. Checked (files: `di01`, `tz13`): a pickup's own lines (picked by the item's kind, event function 644) put their icon after `{0C 04}`, before the item's name (`{0E 01}`), and differ by kind. Now, when the pickup's part calls message function 1 on window 7 (skipped, t806), `chest::line_icon` reads that text (KH1's text table for the script in use, `0x14237C4E0`, walked with the parser's code lengths) and `ingameskip::set_pickup_icon` puts its icon on the pickup's waiting line. The give and the line come in the same script step, before the box is shown at the next pad read.
- Not checked: in game; a line with no `{0C 04}` keeps 0x82 (logged).

### 2026-10-29: Stage 815 (testing branch: Item Pickup: a part another context runs (Deep Jungle's slides))
- **The slides stayed vanilla.** The owner (chat, t812 log `kh1-kh2.prev.log`): "the slides you get at deep jungle are still vanilla". Checked (log): at each slide (f361, f1959, f2087, f2248, f2447, f2602, f2957) "event 1's part (context N, sub-script 6) isn't a pickup itself (event function 0)" and "no sub-script of context N reads as a pickup". Checked (files, `tz13`): the slide's part (511, 2, 644, 258 0xD9..0xDE, 364, window 7 opened once, 353, 107, 2, 319, 377, 512) sits in another context than the touched one (Assumption: the room's main context, by its place in the file), which the slide object's touch part wakes; t810's `read_pickup` counted only sub-scripts of the touched context as the part's own.
- **Now** (`kh1script::read_pickup`, `pickup_parts`, unit-tested): every sub-script the part wakes, of any context, counts as its own; `chest` keeps each context the part runs in with the ranges it runs there, and skips holds and window calls from any of them; the pickup is over when none of them is in its ranges any more (half a second after).
- Not checked: in game.

### 2026-10-29: Stage 814 (testing branch: Reflect's barrier also meets the attack types KH1 meets with nothing)
- **Enemy attacks of clash types 6 and 7 meet the barrier too.** The owner (chat, after the t796 check): "let it meet any guard types it can't like (b)". KH1's pairing table (Steam `0x140528CB8`: `1F 09 09 3F 29 18 00 00`, Checked (exe)) has rows of 0 for types 6 and 7, so KH1's entry test `0x1402B3BE0` meets such an attack with nothing, KH1's own guards included; the testers' logs had type 6 shots near Sora (Checked (trace): 18 of 203 enemy attacks logged). KH2's barrier meets every enemy attack (`0x1403D2140`, Checked (decomp) KH2). `bodyhits::unmet_types`: while KH1's pass builds the barrier's entries, each live enemy attack of type 6 or 7 in KH1's pool is tested against them with KH1's own overlap test (`0x1402B5EF0`, first bytes checked, Checked (exe)) and one that touches goes to KH1's meeting `0x1402B4A20`, where the barrier's rule takes it (`combat::barrier_meeting`: a shot stopped, melee to KH1's halves, counted once per barrier and attack, the contact effect). Its shape: KH1's sphere at its place and radius. Not checked in game. docs/kh1-reflect-spell.md "The barrier as a KH1 guard object".

### 2026-10-29: Stage 813 (testing branch: Reflect's barrier meets clash types 0 to 5)
- **The barrier's KH1 guard is clash type 3.** The owner (chat): "do (a)". t796's pure guard (type 4) met only types 0, 3 and 5 in KH1's pairing table (`0x140528CB8`, Checked (exe)); the testers' logs had enemy attacks of types 0, 1, 2, 4 and 6 (Checked (trace)), so melee of types 1, 2 and 4 passed through the barrier uncounted. Type 3's row meets 0..5. Type 3 also strikes bodies in KH1's entry test (`0x1402B3BE0`, the body branch for types 0, 3, 5, 6 and 7), which skips an attack whose result word has 0x30; KH1's attack pass clears that word right before building the entries (`0x1402B47A0`), so `bodyhits::guard_entries` sets it to 0x10 there every pass: the barrier never strikes a body. The attack-against-attack path reads the result only with the one-hit bit 0x80, cleared on the barrier; KH1's half for an attack already "clashed" (`0x1402B4BC0`) does nothing to it (the barrier stands); a half's "spent" bit 0x100000 is cleared each pass. Checked (decomp). Assumption: an owner whose carrier is switched off (`+0x88` bit 0) makes KH1's pass skip a body-type attack (`0x1402B47A0`): the barrier then meets nothing while Sora is in that state. Not checked in game. docs/kh1-reflect-spell.md.

### 2026-10-28: Stage 812 (testing branch: Item Pickup: Deep Jungle and Hollow Bastion read; a pickup opens one window)
- **The owner** (chat): "everything is working can you now go and look for the scripts for deep jungle and hallow bastion so you can pick up the items quick there as well". Checked (files: every `tz*.ard` and `pc*.ard`, every script version): the parts that hold, give and give back with no scene are Deep Jungle's slides (`tz13`, items 0xD9..0xDE), Hollow Bastion's emblem pieces (`pc05`, 0xBC, 0xBE, 0xBF) and books (`pc06`, 0xB2, 0xB4, 0xB7, 0xB8, 0xB9), all in Destiny Islands' form (511, 113, 115, 644, 258, 364, window 7 opened once with 0, 353, 107, 2, 512, 112, 114), so t811 already takes them the quick way. Also reading as pickups before t812: `tz13`'s Hi-Potion / Ether parts (items 0x2 / 0x3) open window 7 three to eleven times with lines of their own around the give, and `pc06`'s reward 0x18 and spell parts (party held with 593 / 151, Sora's motions, 177 / 605); those are messages and small scenes, not pickups.
- **`kh1script::read_pickup` (unit-tested):** exactly one window opened, with 0; a 177 or 605 on the part's own path makes it not a pickup. Every Destiny Islands pickup still reads as one (each opens once with 0, Checked (files)).
- Left KH1's: Hollow Bastion's 0xBD (`pc05`, given in a small cinematic with no hold of its own), `pc06`'s bookshelf rewards (items 0x4 / 0x9A / 0xFE / 0x6: no hold, already in the small box at (-220, -150)), the books put back (`pc06`, items 0xB2..0xB9 with no hold), Deep Jungle's rewards in `tz15`..`tz20` and `tz11`'s 0xC8 (scenes or no hold). Not checked: in game; where 0xB3 / 0xB5 / 0xB6 come from (no pickup gives them in these files).

### 2026-10-28: Stage 811 (testing branch: Item Pickup: the item goes away at once, the box closes on its own)
- **The Seagull Egg and the Coconut stayed until a button.** The owner (chat, t810 log): "the seagull egg and the coconut doesn't disappear when being picked up, only after you hit a button does it". Checked (log): the Egg's pickup (f4026) skipped its window, but its part then waited on window 7 (107 x 92 / 122 / 66 from f4028) until the jump at f4174, and only then went on (54, 112, 114, 512); the Coconut's the same (f3027..f3200). Window 7 is the in-game skip's "Obtained" box too, and its line ends with `{06}` (0x52 frames, but a wait Confirm ends; Checked (decomp, t804's reading)), so the part waited on our box, and the item goes away after that wait.
- **Now**, from the pickup's own context, KH1's window functions on window 7 take the skip-mode way as 0 / 1 / 177 / 605 do (`chest::PICKUP_WINDOW_FNS`: their values popped, 2 returned; Checked (exe): 2 and 107 pop 1, 3 and 4 pop 3, 5, 6, 80 and 83 pop 2, 365 pops 4), so the part neither waits on, closes nor moves the box; and a pickup's line in the box uses `{05}` (`ingameskip::show_now`: the same 0x52 frames, a wait no button ends), so the box closes on its own.
- Not checked: in game.

### 2026-10-28: Stage 810 (testing branch: Item Pickup: the rope, a pickup its context's main loop does after a touch)
- **The rope still held Sora.** The owner (chat, t809 log): "rope is still not working, its the first item I pick up in the log". Checked (log): f552 "event 1's part (context 44, sub-script 6) isn't a pickup (event function 0)", then f571 the hold (511 x2, 26 x2, 63, 258). Checked (files, `di01`): the rope's context binds object 0x40024 in sub-script 0; its touch part (sub-script 6) only sets local 0 and calls 26; sub-script 1, the context's running loop, sees local 0, holds (511, 113, 115, 26, 63, 511), gives 0xC2, shows window 7, gives back (512), lets Sora get up (wake of context 20's 42) and gives back again (114, 112, 512). The Logs' and the Cloth's touch parts do the pickup themselves.
- **Now** (`chest::event`): when the woken part isn't a pickup, the context's other sub-scripts are read (`kh1script::read_pickup`); one that reads as a pickup is taken as a polled pickup: its holds and windows from that context are skipped and its actions on Sora done at once, as for the others. It's over half a second after its last give-back after the give (`kh1script::releases_after_give`: the rope's two 512s), or after 1.5 s with no give (the touch didn't start it). `read_pickup` now counts sub-scripts of the pickup's own context that it wakes as its own (`kh1script::own_subs` gives their ranges for a pickup's end), unit-tested.
- Assumption: context 20's sub-scripts 42 and 44 (Sora's motions) have no scene, choice or menu (not readable from the files here); if they had, the rope stays KH1's and the log says "no sub-script of context ... reads as a pickup".

### 2026-10-28: Stage 809 (testing branch: Item Pickup: every Destiny Islands pickup form read, the part's end, no second sound)
- **The sound.** The owner: "we don't need to play the item get sfx because the script still plays it". A pickup's line in the in-game skip's "Obtained" window plays no 0x1F now (`kh1script::obtained_script_sounds`, `ingameskip::QUIET`); the pickup's own 353 still runs.
- **Destiny Islands read in full** (the owner: "investigate all the scripts in destiny island and see if we're missing any items"). Checked (files: the 12 `di*.ard` and the 28 `di*.evm`, every script version, identical parts counted once): 36 parts give something. 25 read as pickups (items 0xC0..0xC7 in `di01` / `di03`; 0xC0 Log, 0xC1 Cloth, Checked (log, t805); 0xC2 the rope, Checked (log + files: its part holds twice, as the t805 log's f6472)). Missed by t806: two forms of 0xC5 in `di03` that hold only with the engine's command lock (113 / 115 ... 112 / 114, no 511 / 512) and end at their sub-script's end. Read as pickups by mistake: the raft's building in `di01` (511, 258 x6 taking the materials, Sora's motions, 512, no window opened) would have had its motions done at once. Left KH1's on purpose: `di11`'s 0xC5 (a scene starts after its window, 126), `di03`'s 0xBA (a talk gives it), the potions given in talks (0x1 / 0x2), 0xE7 in a scene.
- **`kh1script::read_pickup` (unit-tested)** now reads the whole sub-script and everything it wakes: a give, 511 + 512 or 113 + 114, at least one window opened and only window 7, nothing in `BOX_PLAYS` and no 514.
- **The part's end.** t806 ended a pickup at its first 512; the rope holds and gives back twice and its getting up comes after. Now a pickup is over half a second after its context has left its sub-script (`kh1script::sub_end`, `chest::pickup_over`), so what it woke is still done at once; a second pickup during that time takes over.
- Not checked: in game; the rope's own touch reason (t806's log line will say if it's still turned down: what its woken context 20 sub-script 44 does wasn't readable from the files).

### 2026-10-28: Stage 808 (testing branch: chest: no chest Open while a box's part runs)
- `win/chest.rs`: a chest's Open is greyed (KH1's own grey and block) while a room script's box opened the KH2 way
  still runs its part (`box_running`: until its 512 or the open's time); a chest opened then never got its
  treasure script (the owner's t805 log).
- `docs/kh1-chest.md` Round 23.

### 2026-10-28: Stage 807 (testing branch: chest: Keyblade trail on room scripts' boxes)
- `win/chest.rs`: KH1's action step's motion request call (`0x14028E8AC`) goes to `action_request` (KH1's request,
  then for a room script's box on Quick with KH2's chest motion, KH1's Keyblade trail request as a chest's);
  the box's requests on Sora logged.
- `docs/kh1-chest.md` Round 22.

### 2026-10-28: Stage 806 (testing branch: Item Pickup: the in-game skip's small "Obtained" box, touches logged)
- **The box.** The owner (chat, t805 log): "the text box needs to be the same size as our in game cutscene skip text boxes so it doesn't obstruct gameplay". Checked (files, `di01`): a pickup opens KH1's window 7 set up centred (event function 4 with 0xF, 3 with (0, 1)) with a long line; the in-game skip's box is window 7 placed at (-220, -150) by 365 with one "Obtained" line (`kh1script::obtained_script`). Now, from the pickup's own context only, KH1's message functions 0 / 1 / 177 / 605 take their skip-mode way (`chest::call`: their arguments popped, nothing opened, 2 returned; Checked (decomp): 0 and 177 pop 1, 1 and 605 pop 2), so its 107 waits pass and its close (2) closes nothing; the give (258 / 583) goes to `ingameskip::queue_obtained`, which shows it in the in-game skip's "Obtained" box at the next pad read with KH1's treasure machine free (`show_now`, also run with In-game Scene Skip off now). KH1's message functions are wrapped once for Chest Opening and Item Pickup together (`wrap_messages`).
- **The rope.** Checked (log, t805): the Logs and the Cloth were read as pickups through event 1 (touch, f5867 / f6057 / f7070) and Sora was free; at f6472 a pickup (511 x2, 26 x2, 63, 258) held him with no Item Pickup line. Checked (files, `di01`): the rope's part (item 0xC2) holds twice (511, a wake of context 20's sub-script 44, 26, 113, 115, 63, 511) and gives the player back twice. Assumption: it comes as a touch too, and was turned down quietly (t801 kept a touch's ways out out of the log). A touch's "not woken", "not a pickup" and "unreadable" are logged now; the next log names the reason.

### 2026-10-28: Stage 805 (testing branch: chest: t804's window wait now runs)
- `win/chest.rs`: the item window's unskippable wait (t804) now runs: message function 1's script stack index is 1
  when the window and text are all it holds (t804 required 2). Logs added for an unreadable line.
- `docs/kh1-chest.md` Round 21.

### 2026-10-28: Stage 804 (testing branch: chest: the item window closes on its own)
- `win/chest.rs`: Quick / Skip: the open's message line (function 1) reads our copy of its text with KH1's unskippable
  wait (`{05}`, frames + 1,000,000) in place of `{06}`, so Confirm no longer closes the item window; KH1's text back
  at the close (2) or the open's end. The Dalmatians' line uses `{05}`. `timed_copy` steps the text with the
  parser's code lengths (`0x1401767B0`), test added.
- `docs/kh1-chest.md` Round 20.

### 2026-10-28: Stage 803 (testing branch: chest: KH2 animation's trail and item window timing)
- `win/chest.rs`: Quick with KH2's chest animation: KH1's trail request (`0x1402C2310`, row 13's trail id) is asked
  when the open request plays KH2's chest motion, so KH1's trail step draws the motion's trail window 7.0 (0..23);
  the item window's wait moves to 20 frames after the lid opens (`LID_TO_WINDOW`, vanilla's gap).
- `docs/kh1-chest.md` Round 19.

### 2026-10-28: Stage 802 (testing branch: chest: lid at KH2's contact, KH2 Keyblade sound (t784 redone))
- Redoes t784 (written over by a later t785 built on t783) on top of t801.
- `win/chest.rs`: Quick with KH2's chest animation: Sora's motion wait (103, treasure context 1) ends at frame 28
  of KH2's motion (`KH2_CONTACT_FRAME`: KH2's lid and capture start together, `0x1404025C0`; Keyblade sound
  500034 and the lid's sound at 28), so KH1 opens the lid at contact; the idle request (58) waits until the motion
  has played out.
- `win/kh2slots.rs`: the chest slot keeps only KH2's sounds; `frames_for_motion`.
- `win/kh2sound.rs`: the chest motion's sounds always cut and allowed.
- `docs/kh1-chest.md` Round 18.

### 2026-10-28: Stage 801 (testing branch: Item Pickup: woken pickups waiting at a priority, touches and event 3 read, every way out logged)
- **Destiny Islands' pickups still held Sora.** The owner (chat, t800 log): "recorded in the log in destiny island I still get stopped for picking up an item". Checked (log): `item_pickup = skip`, the dispatch hooked at f57, Cutscenes logging on (Chest Opening lines show), yet no "Item Pickup" line at the pickups (f1909, f2151, f2454, f2575: 511, 113 / 115, 258 and Sora held). t795's `chest::event` left without a word when the part wasn't seen as woken. Checked (decomp): KH1's wake (`0x1401A0AF0`) sets the context's next instruction only when the wake's priority is at least its current one (`+0x32C`); otherwise the start waits in `+0x30C + priority * 4` until that level ends. t795 took only the first. Now a start waiting in the slot of the wake's priority counts too (event 0 and 4 at 3). Also read now: event 1 (a touch, `0x140297E80`, sub-script 6 at priority 2) and event 3 (`0x1402B0610`, sub-script 7 at 4), in case Destiny Islands' items come that way; `read_pickup` still decides. Every way out for events 0, 3 and 4 is logged (no context, no sub-script, not woken with its next and waiting instruction, not a pickup, during an open). Assumption, not yet checked: which of these the island's items take; the next log says.

### 2026-10-28: Stage 800 (testing branch: KH2 spell scripts: KH2's calls through KH1 call tables on the record)
- **KH2's calls are a KH1 call-table list on every record of ours.** The tester (kh2-magic-and-reflect-native m12 question 1 "(a)", "go ahead"): "...with KH2's script calls added to KH1's table". KH1's call instruction (class 11 in `0x1402CC620`) takes its table from the record's `+0x38` list by bits 6..7 and an entry of 0x10 by its operand (function `+0`, flag word `+8`: argument count, 0x40000000 when it returns); KH1's reset `0x1402BCFD0` gives every record the list at `0x140529FA0`; `0x1402BD5B0` sets none (Checked (decomp), Checked (exe)). `kh2script::call_tables` builds KH2's tables 0..10 in that layout once (every call of `kh2_bdx::traps::TRAPS`, entry `kh2_bdx::run_call` with KH2's flag word), puts the list on each of our records and gives the record its old list back when it ends; a record of KH1's found with our list gets KH1's own before KH1's instructions run. `kh2_bdx::Vm::calls`: the call instruction (class 10) looks its call up there and calls the entry with the arguments' words, the result in the first, as KH1's does (`run_call` runs it through the host's `syscall`). Unit-tested (`calls_through_a_host_table`) and in `native_host_runs_every_spell_as_our_arena`. What each call does is unchanged. Not checked in game. docs/kh1-magic-native.md §8.

### 2026-10-28: Stage 799 (testing branch: KH2 spell scripts: their threads as KH1 child processes)
- **A thread a KH2 spell script starts is a KH1 child process.** The tester (m12 question 1 "(a)"). `trap_magic_start_thread` (Blizzard's and Thunder's scripts, one each, Checked (files) by tracing every level) now asks the host (`kh2_magic::NativeHost::thread`): `kh2script` starts a child with KH1's own `0x1402BD4D0 (parent, code)` (its own record and stack, flag 4, the parent's program and work memory, appended to the chain at `+0x740`; first bytes `48 83 EC 28 48 8B C2 4C 8B D1`, Checked (exe)); the thread's context sits on the child's record, and KH1's `0x1402BCEC0` steps it after its parent (it reads the chain after running the parent, so a child started that step runs that step, as our runtime ran a new thread). A thread that ends returns KH1's "ended"; KH1 frees it with its parent. `Magic::step_external` takes the context (the main one runs its tree, a thread runs alone), the cast is over once its main context and every thread are done (`host::run_ctx`). Only while more than 8 of KH1's 64 records are free (KH1's allocators check nothing); otherwise the thread runs in its parent's step as before. Offline: same attacks, effects and strikes frame for frame (`native_host_runs_every_spell_as_our_arena`, the fake host's threads on child records). Not checked in game. docs/kh1-magic-native.md §8.

### 2026-10-28: Stage 798 (testing branch: KH2 spell scripts as KH1 script processes, stepped by KH1)
- **KH2's spell scripts run as KH1 script processes on KH1's interpreter.** The tester (m12 question 1 "(a)": "Teach KH1's script machine KH2's instruction set, so a KH2 spell runs as a KH1 script process", "go ahead"). New `kh2script`: a cast (Fire, Blizzard, Thunder, Cure, Reflect, Wisdom's casts) starts with KH1's spell start `0x14028F5E0` on its KH2 code (each program loaded once into a block of ours), so KH1 makes the task (spell task list `0x142D22D08`, stepped every frame by `0x14028F6B0`, not while paused) and the process (one of KH1's 64 records at `0x142DC7BE0` and 32 work slots at `0x142DB7BE0`; free ones checked first, KH1 checks none). KH1's interpreter `0x1402CC620` is hooked (prologue `4C 8B DC 49 89 5B 20 55`, Checked (exe)): a record whose program is in our block runs KH2's instruction set (`kh2_bdx`'s decoder, the registers moved in and out of the record every step: pc, the stack in the record, KH2's 8-byte evaluation entries with their "@INT"/"@FLT"/"@ADR"/"@???" tags, the work slot), any other KH1's. The scripts' memory is a 128 KB block per cast that KH1's handles address. `kh2_magic`: `NativeHost` (machine, started, load, store, alive, ended), `Cast::external`, `Magic::step_external` (KH1's step of a cast, with the world our last frame step built, Sora's place read again, `reflect::refresh`); `update` no longer runs those casts; the effects age once per game frame before its first script (`Magic::frame`), and an attack freed in a KH1 step stays one update so our step still forgets its KH1 object (`freed_seen`). A cast with no room in KH1's pools, or any failed check, runs on our machine as before; a process KH1 drops (room change, reset, 30 of our steps without a KH1 step outside the pause) ends its cast. Checked (decomp): KH1's machine, allocators, task list and pause; KH2's interpreter `0x14041B400`. Offline test `native_host_runs_every_spell_as_our_arena` (every level of Fire, Blizzard, Thunder, Cure, Magnet, Reflect from the tester's extracted files): identical attacks, places, effects and strikes frame for frame. Not checked in game. Assumption: where KH1's spell task list runs against our frame step (logged: "KH1 first stepped cast ..." with the game frame). docs/kh1-magic-native.md §8.

### 2026-10-28: Stage 797 (testing branch: KH2's script machine on host memory)
- **`kh2_bdx` can run on memory the host owns.** First step of the tester's m12 question 1 "(a)". `Memory::native(Native { resolve, block, size, used })`: addresses are the host's 32-bit handles, every access goes through the host's resolver (a range outside script memory fails as `BadAddress`, never a stray read), `alloc` hands out the host's block. `Vm::on_memory` (the program already in host memory), `Vm::spawn_at` (a context's stack and work memory where the host says), `Vm::prog_base`. `kh2_magic` re-exports `kh2_bdx`. Unit test `runs_on_host_memory`. Nothing changes in game with this version alone. docs/kh1-magic-native.md §8.

### 2026-10-28: Stage 796 (testing branch: Reflect's barrier as a KH1 guard object)
- **Reflect's barrier is a KH1 guard object KH1's own collision pass meets.** The tester (kh2-magic-and-reflect-native m12 question 2 "(a)", "go ahead"). `combat::Strikes::barrier_guard`: each standing barrier is a KH1 attack object of Sora's made as the body guards are (KH1's row maker on a copy of his swing row with ATKP 573's reflect arc, clash type 4 = a pure guard KH1 never meets with a body, priority 100, always live); KH1's builder (`bodyhits`' hook on `0x1402B3800`) puts in the barrier's own shapes as KH2's ellipsoids (`bodyhits::guard_shapes`). KH1's pass (`0x1402B4460` → `0x1402B3BE0`, its pairing table) finds every enemy attack that meets it; at KH1's meeting `0x1402B4A20` (`recoil`'s hook, now first `combat::barrier_meeting`): a free-standing shot is stopped as KH2 stops one (t582's `stop_shot`) without KH1's halves (they would turn it back); anything else goes to KH1's halves `0x1402B4BC0` (stopped, its owner's clash record, the barrier standing); each meeting counts on the barrier and plays the contact effect where the entries met, once per barrier and attack. The object goes with the barrier, and while it is hidden waiting for the combo. Our own overlap test (`barrier_meet`) only where those hooks aren't in. Checked (decomp) KH1 and KH2 Steam. Assumption: KH1's pairing table lets a type 4 guard meet every enemy attack type the old test met (the body guards rely on it). Not checked in game. docs/kh1-reflect-spell.md "The barrier as a KH1 guard object".

### 2026-10-28: Stage 795 (testing branch: QoL > Item Pickup: an item picked up off the ground leaves Sora free)
- **New QoL row, Item Pickup (Vanilla | Skip; `item_pickup`, Vanilla by default).** The owner (chat): "when you collect an item off the ground the game locks you and won't let you move until you press a button to move the dialog, can you make a new setting in QoL that lets you toggle the ability for when you collect an item it doesn't stop your movement and the text box that appears is the same as our chest and in game cutscene settings?". Checked (log, the owner's t794 log, Destiny Islands `di01`): at f3289 the pickup's part ran 511, 113 / 115, 258, 644, 364, window 7, 353, 107 with "runner 0x2, engine 0xc" and "Sora can act: false" until f3354. Checked (decomp): Triangle command kinds 4 and 5 (`0x1402A3DB0`, `0x1402A4150`) queue events 0 / 4 on the object (`0x14028DD80`), flushed into KH1's event dispatch `0x1401A1A10`, which wakes the object's context's sub-script 5 (event 0) or 3 (event 4); no other code queues those two events. Checked (files, all 239 room files): the parts that read 511, a give, window 7 only, 512, with no scene, choice, shop or menu are Destiny Islands' gathered things (`di01` / `di03`, items 0xC0..0xC7) and parts in `tw*` (0xD3), `pc05` / `pc06` (0xB2..0xBF), `tz13` (0xD9..0xDE), `lm*`, `pp10`, `po*`, `pi37`, `nm01` / `nm11`, `he01` / `he04`, `ew22`, `aw*` (about 440 parts; which world each room code is: Assumption). On Skip, `chest::event` wraps the dispatch: after KH1's own wake, the woken sub-script is read (`kh1script::read_pickup`, unit-tested: hold, give, release, every window it opens is 7, nothing in `BOX_PLAYS`); a pickup gets the evidence boxes' treatment (t769 / t776): its 511 / 512 / 112..115 skipped (only from its own context), actions on Sora to KH1's done queue (no crouch motion), his motion waits passed, and its give, its "Obtained" window 7, its 353 sound and its flags run as KH1 wrote them, so the box is the one chests and the In-game Scene Skip show. Anything else is KH1's. Vanilla: the hook passes straight to KH1 (installed only once the row is first set to Skip). Assumption: KH1's item window doesn't hold Sora (as with chests). Native: KH1's own script, window and give run; only the holds are skipped, as chests.

### 2026-10-28: Stage 794 (testing branch: In-game Scene Skip: a held give tail is skipped, the mod's window shows)
- **The Entry Pass's tail is skipped: Sora is free and the mod's "Obtained" window shows.** The owner (chat, t793 log): "wasn't able to move during that scene, it also didn't play our custom pop up text, but the vanilla one". Checked (log): t793's 512 ran at the 514 (f4705) and the runner word was 0 from f4706, yet Sora couldn't act until about f4770 while KH1's window was up. Now `ingameskip::give_tail` gives such a tail KH1's skip the way a follow-up gets it (bit 0 of `0x142382594` and the flag byte `0x14232DF89`; KH1's script step `0x1401A2320` starts it with `0x1401A22B0`, which needs no event mode, Checked (decomp)), so the give runs at once, KH1's window and sound don't play, and the tail's own 512 ends the skip (`end_skip`) and shows the mod's window with 0x1F. Event function 258 (`0x1401AC700`, item one below the top, count on top, Checked (decomp)) is now wrapped in the table like 583 / 458 / 444 / 570, so its item is noted. Assumption: KH1's skip start fades out for the few frames up to the 512 (a short dip to black).

### 2026-10-28: Stage 793 (testing branch: In-game Scene Skip: a scene's held give tail lets Sora go)
- **The Entry Pass no longer holds Sora.** The owner (chat, t792 log): "after you get thunder then go to get the entry pass the entry pass still locks your movement and plays like vanilla". Checked (log + files): the Coliseum's first room (`he01` version 1), entry 1: context 1's pause-skippable part (the Cutscene Skip row's, f12426) ends with 140 and its sub-script, so t765's section-end reading found no end ("no 514 or 512", f12457); the rest is context 0's: 514, a wait, 258 (item 0xE5, the Entry Pass), 364, window 7 (4 / 5 / 6 / 83 / 80, 3, 0, 1), 353 with 0x1F, 107, 2, then 512, with the player held since its 511 (runner bit 1). `ingameskip::event_off_fn` now reads, at each 514 with the row on, no skip of the mod's running and the player held, the context's rest up to its 512 (`kh1script::read_give_tail`, unit-tested: a give 258 / 583 / 458 / 444 / 570, no choice, shop, menu or pause skip, no further 514); if it's such a tail, KH1's own 512 (`0x1401A7660`) runs right there, so Sora is free while KH1's give, window and sound run as written (as a chest's box, `chest.rs` t769 / t776). Assumption: the window doesn't hold him.

### 2026-10-28: Stage 792 (testing branch: In-game Scene Skip: the window's free check reads KH1's task list)
- **The "Obtained" window no longer stops coming after a room change or a soft reset.** The owner (chat, t791 log): "still not working ... I think soft resetting is not resetting the flags for our script to work properly". Checked (log): the Blizzard window started at f1525, the room changed at f1654, Thunder was noted at f6904, its skip ended at f6908 and no window came. t789 / t791 judged the treasure machine by its contexts' "ended" bits, which stay as they were when a room load or a reset drops the machine's task. Now the machine is busy only while its step (`0x1401A2CF0`) has an entry in KH1's script task list (`0x1423D3E18`: entry size u16 `+0`, count u16 `+2`, entries at `+8`; an entry's first u16 is 0 when free and its step is at `+0x10`; `0x14028A900` adds one, `0x14028A710` runs them and frees one whose step returns 4, which `0x1401A2CF0` does when the script ends). Checked (decomp). If the list can't be read, it counts as free.

### 2026-10-28: Stage 791 (testing branch: In-game Scene Skip: the window's free check looks at context 0 only)
- **The "Obtained" window comes after a chest has been opened.** The owner (chat, t790 log): "the pop up didn't happen in my last cutscene but happened earlier". Checked (log): f853 the Blizzard window started; f3400 / f4299 two Dalmatians' chests (the treasure script); f11709 Thunder noted, f11713 the skip ended at 514, the runner word 0 from f11715, and no window. t789's free check waited until every context of the treasure machine had ended, but the treasure script's contexts 1..3 idle in a loop and never end; the machine's task (`0x1401A2CF0`) ends the script when context 0 has ended (Checked (decomp): the step loop's end branch is for context index 0 with bit 3 of `+0x33C`). So after the first chest the window never came again. Now only context 0's bit 3 is checked.

### 2026-10-28: Stage 790 (testing branch: In-game Scene Skip: the give's sound with the "Obtained" window)
- **Each "Obtained" line after a skip plays KH1's get sound.** The owner (chat): "its working, can you add the sound that plays when you get an item from those cutscenes/chests?". The scenes play it themselves with event function 353 (`push 0x1F, push 0, call 353`; `0x1401C8F40` -> `0x1401787D0`; in skip mode only sound 0x3EB0 plays, so a skipped scene's is lost). Checked (decomp + files): across the scripts read, 0x1F is the sound next to the item message (605) 112 times, next to 258 25 times and to 570 13 times, and the Blizzard scene (`aw02`) plays it after its "learned" line. `kh1script::obtained_script` now puts `push 0x1F, push 0, call 353` before each line's window opens (unit-tested), so KH1 plays it as the window shows. The treasure script itself plays 0x1E (its context 2, with the lid), not 0x1F.

### 2026-10-28: Stage 789 (testing branch: In-game Scene Skip: the "Obtained" window's free check)
- **The window now starts.** The owner (chat, t788 log): "logged it still didn't see it". Checked (log): f719 `KH1 gave "Blizzard" during the skip`, f725 the skip ended at 512, runner `0x0` from f726, and no "window(s) started" line. Cause: t786's free check compared the script in use (`0x142382530`) with the machine's file word (`0x1423BBBE0`), but the room script's own start (`0x1401A38A0`) writes both, so they were always equal and `show_now` always waited. Checked (decomp). Now the machine is free when none of its contexts (`0x1423BF0B0`, count `0x1423AB2DC`) is still running: each has bit 3 of `+0x33C` set once its last sub-script ends (the class 5 handler `0x14019FAB0`; the task `0x1401A2CF0` ends when they all do).

### 2026-10-28: Stage 788 (testing branch: In-game Scene Skip: a spell is shown even when its level doesn't change)
- **Blizzard's window now shows on a save that already has it.** The owner (chat, t787 log): "still the same thing visually". Checked (log): f753 `event function 458 during the skip: spell 1, level 1 (was Some(1))`: the wrapper was installed and called all along; the save already had Blizzard at level 1 (a replayed scene), and t785..t787 only noted a spell whose level changed (t787's save comparison likewise). So t787's reading of t786's log was wrong: nothing was missing in the wrapper. Now 458 notes the spell whenever it runs during the mod's skip, as the scene would show its lines anyway. t787's save comparison is taken out again: abilities (583 kind 1) and munny (444, the difference KH1 made) are noted by their wrappers as in t785, so a replayed scene shows them too.

### 2026-10-28: Stage 787 (testing branch: In-game Scene Skip: the gives read from Sora's save data)
- **Spells, abilities and munny a skip gave are read from the save, not from their functions.** The owner (chat, t786 log + trace): "the item text box doesn't appear after it skipped in an game cutscene where you get blizzard". Checked (log + files): Wonderland room 1 (`aw02`, version `0xd`), entry 3 was skipped (f1653..f1673, ended at 512) and event function 458 ran in it (the second's counts at f1674; in the script it is `push 1, push var 0x18, call 458` after 513), but no "KH1 gave" line came: t785's wrapper on 458 never noted it. Not found why (its install line is in the Menus log, which was off; now in Cutscenes). Now the mod keeps Sora's spell levels (`[0x142868BA0] + 0x492`), his ability list (`+ 0x44`, 48 bytes) and munny (`[0x142E1F4D8] + 0x1C`) when its skip starts and compares them when it ends: a spell whose level went up, an ability that wasn't in the list, munny gained (Checked (decomp): `0x14028F560`, `0x140284E10`, `0x1402E7E10` write there). Items (583 kind 0) and Gummi Blocks (570) are still noted by their wrappers. The 458 wrapper only logs its calls (spell, level, whether the mod's skip was running).

### 2026-10-28: Stage 786 (testing branch: In-game Scene Skip: the gives shown in the chests' "Obtained" window)
- **A skipped scene's gives now show in KH1's own window 7, as a chest's "Obtained".** The owner (thread m41, (a), "go ahead"): t785's level-up box didn't match the chests, which since t776 show the treasure script's own window. `kh1script::obtained_script` builds a script file in KH1's form (unit-tested) from the treasure script's own window calls (Checked (files): window 7 set up with 3 / 4 / 5 / 6 / 83 / 80, then per line 365 at (-220, -150), 0, 1, 107, 2) and texts in its "Obtained" form (`obtained_text`: the same leading codes, colour, icon 0x82 for items and 0x86 for abilities, the name, a full stop, `06 52 00` so the window closes itself). When the skip has ended, `ingameskip::show_now` (pad read) starts it the way KH1's event 5 starts the treasure script, `0x1401A43A0 (file, 0)` (first 16 bytes checked; without them t785's level-up box is the fallback): KH1 swaps in its texts, adds the machine's task (`0x1401A2CF0`), shows each line in turn and puts the room script's file and texts back when it ends. Checked (decomp). It waits until the runner word's low four bits are clear (the start clears them; no skip, no hold on the player) and the machine is free (`0x142382530` isn't `0x1423BBBE0`, so no chest's script runs). No holds (511 / 512, 113..115): Sora stays free.
- t785's `menu::queue_notice` call (gone with the chest rework, which made `queue_prompt`) is replaced; `queue_prompt` is only the fallback.
- Not checked in game (Assumption): that a line with no item code (`0E`) and the name written out shows the same as the treasure's (the treasure texts take the item's name from 364); the spell, munny and Gummi lines have no icon.

### 2026-10-28: Stage 785 (testing branch: In-game Scene Skip: an "Obtained" box for what a skipped scene gave)
- **What KH1 gives during In-game Scene Skip's skip is shown after it, in the chests' notice box.** The owner (chat, then auto-cutscene-skipper m39, answers "1 and 2 with (a)", 3 and 4 left): "for in game cutscenes that have items in them like phil giving you thunder I want it to play a text box like our custom chest text box". Event functions 583 (a reward record: an item with its count, or an ability), 458 (a spell's level: only when it changes), 444 (munny, the difference KH1 made) and 570 (a Gummi Block) go through `ingameskip::give_*` (event table; KH1's own runs in every case). While the mod's skip runs (`noting`: the row on, `FORCED`, KH1's skip mode) each is noted (up to 8), and when the skip ends (`end_skip` or `release`) each goes to `menu::queue_notice("Obtained", ..)`, in KH1's order. Names: items `equip::item_name` (as chests), abilities `menu::ability_name` (new: the ability table KH1 reads, `0x142D37748`, name handle `+4`), spells English level names (Assumption: KH1's own text not looked up), munny "N Munny", "Gummi Block". Checked (decomp + exe: the four functions, their stack arguments, the reward table `0x142D36D58`, the spell levels at `[0x142868BA0] + 0x492`, munny at `[0x142E1F4D8] + 0x1C`).
- Numbered 785: 768..783 are the Chest Opening stages, and 784 is the scan speed-up (`Reach`, `crash::readable_span`) already in `ingameskip.rs`, which has no stage of its own here yet.
- Left as asked (m39 questions 3 and 4): scenes the Cutscene Skip row skips get no box, and an ability for Donald or Goofy shows only the ability's name.

### 2026-10-27: Stage 783 (testing branch: chest: KH2's chest animation on Quick)
- `win/pose.rs`: `add_chest_motion` loads the treasure box `F_EX030` as an object and adds its capture slot 252
  (`0010` body, `0011` / `0012` Keyblade) to KH2 Sora's moveset and rig as motion 1252 (`CHEST_MOTION`); its
  files noted for the slot cache.
- `win/kh2slots.rs`: `CHEST` spec, motion 1252 on KH1's open request 0xA6 (template slot 83); `redirect`
  points it there only while `chest::kh2_animation()`.
- `win/chest.rs`: `kh2_animation()` (Quick and the row at KH2); `set_style` / `set_anim_kh2` refresh the
  requests; the open's log says which motion plays.
- `win/config.rs`: QoL > Chest Animation (KH1 | KH2) back, shown only on Quick.
- `docs/kh1-chest.md` Round 17.

### 2026-10-27: Stage 782 (testing branch: chest: chests' facing from +0x30)
- `win/chest.rs`: `chest_touch` reads Sora's and the chest's facing from their direction vectors (`+0x30`, as
  KH1's range test `0x140293FC0` does) instead of the yaw `+0x3C`, which is 0 for every chest (the owner's t781
  log: two of the throne room's three chests were always greyed). The log line prints both vectors.
- `docs/kh1-chest.md` Round 15.

### 2026-10-27: Stage 781 (testing branch: chest: offer diagnosis)
- `win/chest.rs`: `chest_touch` reorganised (KH1's offer first, then the KH2 grey) and logs each time KH1 asks a
  chest, at most once a second: positions, yaws, distance, KH2's facing values, offered / greyed (the owner:
  no prompt appeared).
- `docs/kh1-chest.md` Round 14.

### 2026-10-27: Stage 780 (testing branch: chest: KH2's facing rules for the Open)
- `win/chest.rs`: the chest's touch callback (`0x1402B5A30`) is hooked (prologue checked): on Quick / Skip,
  KH2's box Open rules from the KH2 Steam decomp (`0x1404088F0`, command 0x20's `+0x10` -0.7071 and `+0x18`
  ~0): the chest no more than 135 degrees off Sora's facing and Sora on its front side; otherwise KH1's offer
  is greyed and blocked (slot 3 bits of `0x140528904` / `0x140528908`). The round 12 comment on 0x1A5 is
  corrected.
- `docs/kh1-chest.md` Round 13.

### 2026-10-27: Stage 779 (testing branch: chest: KH2's reach for the Open)
- `win/chest.rs`: Quick / Skip give each chest KH2's reach: the chest tick's one-time call (`0x1402B5879` ->
  `0x1402AE160`) is redirected to `chest_first`, which sets its object record's range shape to 1 (flat and
  height, no facing) and radius to max(own, 185) (KH2's box reaction sphere r150 + Sora's body r35, Open
  0x1A5 with no range / facing check). KH1's own context scan then offers Open while running past.
- `docs/kh1-chest.md` Round 12.

### 2026-10-27: Stage 778 (testing branch: chest: Skip opens from the run)
- `win/chest.rs`: the Open handler's stop (`0x1402B59D7`: `0x140297F30 (sora, 0.0)`, speed zeroed) is
  redirected to `open_stop`: on Skip a fast open keeps Sora's speed, so he runs on through it (the owner: open
  from the run like KH2). KH1 already takes the Open while running (state 0), Checked (exe + traces).
- `docs/kh1-chest.md` Round 11.

### 2026-10-27: Stage 777 (testing branch: chest: one-line Dalmatians window)
- `win/chest.rs`: Quick / Skip, the Dalmatians' window: when the treasure script places text 4 (event
  function 365), the text table's pointer for it (`0x14237C4E0`) is our one line in the "Obtained" texts' form
  ("Found dalmatians no. A, B, C.", KH1's number codes) and 365's x / y become the "Obtained" windows' (-220,
  -150). KH1's pointer goes back at the window's close (2) or the open's end.
- `docs/kh1-chest.md` Round 10.

### 2026-10-27: Stage 776 (testing branch: chest: KH1's own item window)
- `win/chest.rs`: Quick and Skip no longer skip the open's message windows: the treasure script's (and a
  box's) window 7 opens with KH1's own lines and closes itself, as the evidence boxes' (the owner: "exactly
  like" that box). On Quick the window's open (functions 0 / 177) returns KH1's wait (4) until 100 frames
  into the open (`WINDOW_DUE`). The level-up-box notice is gone.
- `win/menu.rs`: the notice queue (only the chest used it) removed. `win/config.rs`: the row's help text.
- `docs/kh1-chest.md` Round 9.

### 2026-10-27: Stage 775 (testing branch: chest: Quick's notice on vanilla's frame)
- `win/chest.rs` / `win/menu.rs`: on Quick the "Obtained" notice shows 100 60 Hz frames after the Open
  (`QUICK_NOTICE_FRAMES`, the owner: vanilla's window frame), through `menu::queue_notice_at` (notices carry a
  due `takeover::TICKS`; the existing per-frame drain skips one not yet due). Skip and boxes unchanged.
- `docs/kh1-chest.md` Round 8.

### 2026-10-27: Stage 774 (testing branch: debug menu: animation panel counts KH1's own frames)
- `win/pose.rs` / `win/kh2slots.rs`: the Anim Display / Frame Advance panel counted no frames on KH1's own
  animations (`Drawn.frames` was never set on the kh1_own route, so the frame stayed 0). It now reads Sora's
  motion time (`+0x16C`, `kh2slots::kh1_time_now`), so the panel shows the frame within KH1's slot and its loops.
- `docs/kh1-debug-menu.md`.

### 2026-10-27: Stage 773 (testing branch: chest: Quick lets Sora go on idle)
- `win/chest.rs`, Quick only: the open keeps Sora's actor flags and applied motion request from before KH1's
  open. `quick_check` (on each of KH1's script calls while the open runs) sees the open motion start and, once
  his applied request is back to idle (0, slot 0), lets him go: `+0x444` bits 0 / 1 off, hold flags 0x1000000 /
  0x200000 / 0x100000 / 0x80 / 0x40 back to their values before the open. After that the open goes the Skip
  way (`quick_holding()`: actions on Sora to the done queue, waits on his motion pass). The backstop also puts
  the flags back. Boxes on Quick use the same rule.
- Why: the tester's t772 trace showed him on idle from f2108 but held until f2159 (the script's own release).
- `docs/kh1-chest.md` Round 7.

### 2026-10-27: Stage 772 (testing branch: Chest Opening Quick: the treasure script's own hold and idle)
- **Chest Opening Quick: the treasure script holds Sora through the animation and frees him on his idle, as KH1 does.** The owner (two traces with t771): "I'm still able to move during the chest animation and when I return to idle I'm stuck in the idle state". Checked (trace `trace_1793161449_f2743`): KH1's open motion `0xA6` stays applied f2567..f2707, held on its last pose; Sora moves from f2600 (t771's one-second fallback let go) while `0xA6` is still on, and KH1 only switches to his run at f2708. Checked (files, `UK_treasure.ev`): the motion has no end of its own; the script's context 1 (actor 0, Sora) holds him (55; 336 / 338, 57 and the wait 103 in sub-script 11), then asks for his idle (`push 0, push 4, call 58`: action 0x11, request 0, blend 4, `0x1401A7F80`, Checked (exe)) and unlocks him (56; then 56, 54, 335, 337). t768..t771 sent those actions to KH1's done queue, so on Quick nothing asked for his idle. Now on Quick the script's actions on Sora are KH1's (`chest::to_done` on Skip only), the motion wait is KH1's, and the mod's own idle check and the box's own motion request are gone; only the windows (skip mode) and the holds 511 / 512, 112..115 are skipped. Backstop: a lock (`+0x444` bits 0 / 1) still on at the open's end (8 s, or a box's 512) is cleared and logged. Not checked in game. Notes: `docs/kh1-chest.md`, "Round 6".

### 2026-10-27: Stage 771 (testing branch: Chest Opening Quick: held until idle; Chest Animation row off)
- **Chest Opening Quick: Sora stays put until he is back on his idle; the Chest Animation row is off for now.** The owner: "(c) uncomment the code for the animation switch, we'll find it later. For now make you unable to move during the chest animation until you return to the idle animation for the quick setting". Quick no longer lets Sora go at the open motion's end: the wait on his motion (event function 103) going on marks it over, and the next script step that finds his applied request (`+0x164`, low 16 bits) on KH1's idle (request 0, `xa_ex_0010.mset`, Checked (files)) clears KH1's lock (`chest::quick_release`); a second after the motion's end without idle, or the open's 8 s, lets him go anyway (logged). The Chest Animation row and QoL's switch for it are commented out in `config.rs` (the row's code and `chest_anim` stay). Checked (files + decomp): field Sora's `P_EX100.mset` has no chest motion (every non-empty motion accounted for), and his command handler (`0x140404FD0` -> `0x140405440` -> `0x1403A8B10`) passes Open (`0x1A5`) to the chest; only Atlantica's Sora (`0x140405ED0`) plays one (`0x98`). Not checked in game. Notes: `docs/kh1-chest.md`, "Round 5".

### 2026-10-27: Stage 770 (testing branch: Chest Opening: Vanilla | Quick | Skip, and Chest Animation on Quick)
- **Chest Opening is Vanilla | Quick | Skip, with Chest Animation under it on Quick.** The owner: "make it a three setting toggle. Vanilla (formerly KH1), quick (formerly kh2), Skip ... Skip should be currently how kh2's logic works where no animation plays and your able to move right after you open the chest ... if your on quick however a chest animation plays and when you have it selected as quick another hidden config in the menu appears that lets you choose the animation kh1 chest animation kh2 chest animation". QoL's row is a three-value row (`Pick::Chest`, ini `chest_style = vanilla | quick | skip`; the old `kh1` / `kh2` read as vanilla / skip, skip by default). Vanilla is KH1's; Skip is t768 / t769 as they were. Quick: the Open handler's request for KH1's open motion (`0x900A6`, its call `0x1402B59CC`) goes through and KH1's lock (`+0x444` bit 1) is kept; the treasure script's wait on Sora's motion (event function 103, `0x1401A96F0`) is KH1's own, and when it goes on the lock is cleared (`chest::quick_release`); a room script's box gets the same request and lock at the pick, let go at its wait or its 512; the 8 s window's end lets go too. Assumption: the open motion plays whole with the lock on.
- Chest Animation (KH1 | KH2, ini `chest_anim`) is QoL's sixth row, there only on Quick (`List::items`); our list defs take their row count when made (`config::create_list`) and a change that shows or hides a row makes the list again in place (`config::rebuild`). KH2's value plays KH1's open motion for now (logged): KH2's Open is a message to the box, which plays its own `0x98` (Steam KH2 `0x140405ED0` / `0x140406020`, set-up `0x140402200`); no player code asks Sora for a motion on it. Checked (decomp + exe). Not checked in game. Notes: `docs/kh1-chest.md` "Round 4", `docs/kh1-config-menu.md` 6.z7.

### 2026-10-27: Stage 769 (testing branch: Chest Opening KH2: the room scripts' boxes (Wonderland's evidence))
- **Chest Opening KH2: the room scripts' boxes (Wonderland's evidence).** The tester (kh2-chest, after m3): "I also found another chest type which is the evidence boxes you find in alice's world". Checked (log, f45532): a Triangle pick in `aw05`, then the room script holds Sora ("runner 0x2, engine 0xc", "Sora can act: false") to f45665; no Open handler call. Checked (files): the boxes are parts of the room's own script, offered as command 0x55 by event function 454 and run from the object's context's sub-script 9 after `push 0x55, 1/6, 3/<skip>`: 511, 113 / 115, Sora's open motion (`push 0xA6, call 108`) and its wait (103), the item (`push 0xDF, push 1, call 258`), a message, 112 / 114, 512. Checked (decomp + exe): KH1's pick `0x1401A4D00` finds the object's context (room list `0x142382AD0` / `0x14237F4E4`, or the event list `0x1423902D0` / `0x14237F4E8`), wakes sub-script 9 (`0x1401A0AF0`) and keeps the command.
- Now KH1's pick is hooked (`chest::pick`, first 15 bytes checked); for command 0x55 or 0x49 on KH2 the woken part is read in place (`crate::kh1script::read_box`, unit-tested): a part that takes the player (511), gives (258 or 583) and gives him back (512) with no scene, choice, shop or menu is a box, and its contexts get the chest's treatment until its 512 (holds skipped, actions on Sora to KH1's done queue, motion waits passed, windows in skip mode), with the notice from its give (`chest::box_notice`). Read across the 236 room files: 0x55 in `aw02`, `aw05`, `ew02` and 0x49 in `aw02` are boxes; Traverse Town's 0x72 reads the same but is left KH1's (Assumption: the mailbox). Everything else is KH1's. Not checked in game. Notes: `docs/kh1-chest.md`, "Round 3 (t769)".

### 2026-10-27: Stage 768 (testing branch: Chest Opening KH2: the treasure script's holds on Sora skipped, its actions on him caught)
- **Chest Opening KH2: the treasure script no longer holds Sora.** The tester (kh2-chest, after m3): "still unable to move on the chests we've modified" (t743 log, `trace_1793085402_f43568` and three more). Checked (log + trace): no "done at once" line at any open, only the motion-wait one; "runner 0x2, engine 0xc" in the watch line from the frame after the Open; Sora's flags `0x0130c0c3` and KH1's open motion `0xA6` from f43411, then idle with the stick held. Checked (files): `UK_treasure.ev` entry 0 holds the player with event functions 113 / 115 (its first sub-script, run at the script's start), 511 (its main part's first step) and actions naming its context's actor, which starts at 0. Checked (exe + decomp): 511 `0x1401A7540` sets the runner's bit 1 (KH1's game mode "event", `0x14017EFC0`), 113 / 115 `0x1401A9A80` / `0x1401A9AA0` set bits 8 / 4 of `0x142867364` (bit 4 stops Sora's command input, `0x1402937F0`); an action's actor 0 is the party's first (`0x140291D30`). t728 matched actions on Sora's `+4`, so none was caught.
- Now `chest::to_done` matches each queued record through KH1's own lookup `0x140291D30` (its first bytes checked); from a treasure context during a fast open, event functions 511, 512, 112, 113, 114, 115 (no arguments) are skipped; and the Open handler's call of the motion request for `0x900A6` (`0x1402B59CC`, Checked (exe)) goes to `chest::open_request`, which asks for nothing on a fast open (Sora keeps his motion), else KH1's request. t729's swap of the pending request is gone (it never logged in the t743 log). Assumption: nothing else holds him. KH1 setting: nothing installed, as before. Not checked in game. Notes: `docs/kh1-chest.md`, "Round 3".

### 2026-10-27: Stage 767 (testing branch: In-game Scene Skip: the Cutscene Skip row no longer drops its flag byte)
- **The Wonderland scene's line-by-line part now gets KH1's skip.** The owner (chat, t766 log): "the cutscene still did not skip". Checked (log): f423 `... after its skippable section (event function 140) (context 0, instruction 193, 514 instructions read) skipped: bit 0 and the flag byte set`; from f424 the second flag word stays `0x401` (bit 0 on) but the runner stays `0x2` (no skip mode) until f1563 (`event function 512 reached before KH1's skip started`). The flag byte was gone: the Cutscene Skip row (`cutsceneskip::step`) had fired on the section (f397), and at its next pad read bit 0 was the mod's (`ingameskip::forced`), which it reads as "the scene is over" and drops a fired, unused flag byte, ours. Now it leaves the byte alone while `forced()`. And `ingameskip::step` sets bit 0 and the byte again if the skip hasn't started and the byte is gone (logged, at most 4 times).

### 2026-10-27: Stage 766 (testing branch: In-game Scene Skip reads longer follow-ups; never into a pause-skippable section)
- **The Wonderland scene's lines are read in full.** The owner (chat, t765 log): "same issue on the same scene". Checked (log): f412 `entry 1 goes on in event mode after its skippable section (event function 140) (context 0), plays (its code couldn't be read)`. The cause was t764's reader limit: `kh1script::FOLLOW_SUBS` was 16 and this scene wakes 18 sub-scripts (`aw05` version `0xd`, entry 1). It is now 128; the most any section end or room-script follow-up wakes in the scripts read is 40. Checked (files). The log line now names the limit.
- **A follow-up stops at a pause-skippable section.** The owner: "the goal is to not skip high poly scenes ... 'high poly cut scenes' [are] cutscenes that are affected by our 1st cutscene skip toggle ... in game cutscenes [are] the ones like phil giving you thunder and you have to manually read and scroll through the text bubbles". The follow-up reader (`skip_follow_up`) now treats the pause skip's functions (84, 139, 476, 489, 623) like a choice: a follow-up with one ahead plays, so In-game Scene Skip never runs KH1's skip into a section the Cutscene Skip row owns. None of the 112 section ends read has one ahead. Checked (files).

### 2026-10-27: Stage 765 (testing branch: In-game Scene Skip: the part of a scene after its skippable section)
- **The low-poly lines after a high-poly section are skipped.** The owner (chat, with the t764 log and `trace_1793144221_f30815`): "there's also in game cutscenes in alice's world that aren't skipping". Checked (log + files): Wonderland's room 4 (`aw05`, version `0xd`), entry 1, context 0: event function 139 (the pause skip on), the high-poly part, 140 (its end), then 18 sub-scripts woken in other contexts with lines confirmed one by one (0 / 1 / 177, waits 33 / 107), no choice, and 512, 514 at the end. The Cutscene Skip row skipped the section (f30680) and the lines played from f30723; In-game Scene Skip had judged the entry the Cutscene Skip row's (139) and never looked again. Event functions 85, 140, 490 and 624 (the four section ends, `0x1401A8BD0`, `0x1401AA120`, `0x1401B05F0`, `0x1401A8C00`) now go through `ingameskip::section_end`: KH1's own, then, with event mode still on and no talk bit, the rest is read from the next instruction like t764's follow-up (`read_follow_up`: to 514 / 512, woken sub-scripts whole) and skipped the same way. Across the scripts read: 112 section ends, 84 with an end found and no choice, 28 without (they play), none with a choice. Checked (files).
- `end_skip`: a 514 / 512 reached before KH1's script step has started the skip (a tail run in the same step as the section end) takes the bit and the flag byte back, so no skip starts in play.
- The `in-game skip: entry 1 (a scene) plays (event function 88 ...)` lines in Wonderland's room 3 (`aw04`) are its exit to room 4 turning event mode on inside the room's long-running entry 1 (talk handlers with a choice, no scene of its own): nothing was missed there. Checked (log + files).

### 2026-10-27: Stage 764 (testing branch: In-game Scene Skip ends its skip at 512 / 514: the tournament black screen, the room script's follow-ups)
- **The Coliseum tournament should no longer go black (not run in game).** The tester (auto-cutscene-skipper, m34): "you get stuck to a black screen during Olympus coliseums tournament" (log + `trace_1793085779_f64663`). Checked (log + trace + files): each round is an event entry (`he06`): a short lead-in, event function 514, 512 (Sora given back), the fight, then the next entry through 126 from the entry. t712 skipped the lead-in with KH1's skip, which KH1 only ends at its section ends (event functions 85/140/490/624) or back at entry 0, so the fights ran in skip mode (runner odd, frame scale 4, 4 steps a frame in the trace) behind the fade out. Event function 512's table entry (`0x1404FA870 + 512 * 8`, `0x1401A7660`) now points at `ingameskip::release_fn`: KH1's 512, then, while the mod's skip runs, what 140 does (bit 0 off, the flag byte cleared, KH1's `0x1401A2130`, 6 for 2). Read in 110 script files (38 rooms, nine worlds): in the entries the row skips 512 is the last step but in the Coliseum's fight entries.
- **The room script's follow-up after a scene is skipped (the tester's (b)).** Back at entry 0 with event mode on and no talk bit, the context that started the entry (`push n, call 126` before its `+0x190`) is read to its first 514 / 512 with every sub-script it wakes (`crate::kh1script::read_follow_up`, unit-tested); no choice, shop or menu and an end found: bit 0 and the flag byte, KH1's skip, ended at 514 (`ingameskip::event_off_fn`, `0x1401A7750`) or 512. The Thunder lines after Phil's scene (`he02`) are such a follow-up. The doc's earlier "KH1 can't leave skip mode mid-scene" is corrected (`docs/kh1-ingame-skip.md`).
- Steam only, checked before use: `0x1401A2130`'s first bytes and event function 140's call of it, the two table entries. Not native in one way: the end at 512 / 514 is the mod's addition to KH1's own ends. Off: nothing more than t712's one load; the two wrappers pass straight through.

### 2026-10-27: Stage 763 (testing branch: Anim Display, the animation panel without Frame Advance)
- **A debug menu row shows the Frame Advance animation panel on any frame.** The owner asked for the animation readout Frame Advance shows to be a toggle regardless of Frame Advance. Main page > Anim Display `< On / Off >` (`debugmenu::anim_display`) draws `fapanel`'s box (animation, how it is drawn, frame / length, loops, trace frame) in the top right corner with Frame Advance on or off; its title is ANIMATION without Frame Advance, FRAME ADVANCE with it. Checked (files): the draw routes record `pose::drawn` without looking at Frame Advance, so the panel reads the same numbers either way; not checked in game. Session only (not in the ini), off with the Config menu's Debug Menu row; nothing runs while it is off beyond one flag read. Not tested in game. Notes: `docs/kh1-debug-menu.md`.

### 2026-10-27: Stage 762 (testing branch: a Drive Form lands with its air speed)
- **A Drive Form lands with its air speed, not the speed it had before the jump.** The owner, adding to drive-forms with a go-ahead (traces `trace_1793133757_f9084`, `trace_1793133763_f9407`): Wisdom is "not following the stick direction when he lands". Checked (trace): on all eight landings the slide on KH1's landing (request 0xC, Wisdom's 5) was the ground velocity `shared::physics::ground_inertia` kept from before the takeoff (f8865 (-1.83, 6.48), then f8905 (-1.80, 6.38) on landing, while his air speed at f8904 was (7.60, 4.10), steered by the stick; the same at f8975, f9049, f9200, f9267, f9339). `GROUND_V` was only ever set back outside a form, so the landing ignored where the stick had taken him. KH2 has one velocity (`+0x20`) for the air and the ground (Checked (decomp)), so its landing goes on from the air speed. Now `air_step` clears `GROUND_V`, and the first ground step takes KH1's air speed (`+0x3C0`, as the existing fallback does). Unchanged, as KH2: during the landing's frames 0..11 KH2 clears the stick target (`0x1403A89A0`: kept only with range ids 0 / 1 / 2 or 41; Wisdom's 5 has range 2 from 12 and 42 all through, Checked (files)), so the inertia (`0x140422350` -> `0x1403B7AD0`) brakes that speed and he turns from frame 12. Not checked in game.

### 2026-10-27: Stage 761 (testing branch: Valor's left Keyblade kept in his hand between moves)
- **Valor's left Keyblade stays in his left hand between moves.** The tester (drive-forms, after m2): "the 2nd keyblade is still disappearing when you move from one animation to the next". `leftblade::left_draw` draws the left Keyblade only with a holder pose from KH1's pass in the last 2 game frames (`posed_left`), else KH2's own left pose (`LEFT`), which nothing feeds on KH1's states since t702. So every frame without a fresh holder pose drew no left Keyblade: a frame KH1 posed a slot with no holder motion (KH1's own slot between two moves), a holder blend refused by its heap check (`pose_holder` returns before posing), or a pose that isn't finite. **Assumption**: which of these the tester saw (no log of it; the t749 log in the game folder shows the holder posed and blended, 3 lines each). In KH2 the left weapon always follows its joint (bone 86, `sklt`, Checked (files)). Now `posed_left` keeps the last good hold (the holder's bones from its left hand, `LAST_HELD`) and puts it on his left hand as KH1 draws it on such frames, so the blade never leaves the hand; a log line (Animation, 20 at most) says why the holder pose was missing and KH1's slot then. Cleared when Valor ends. No per-frame cost beyond the 13 matrices kept. Not checked in game.

### 2026-10-27: Stage 760 (testing branch: Valor's ground slam effect: pppRandDownFV)
- **Valor's ground slam effect plays: KH1 runs pppRandDownFV as KH2.** The tester (drive-forms, after m2): "soras square finisher where he hits both keyblades on the ground are missing its effect that has a launch hitbox on it". Valor's 155 (row 12, its Square step, and row 3) starts elements 13 and 14 at frame 0 and 15 at 42 (Checked (files), `P_EX100_BTLF.mset`); element 15 (effect 9 on the object root, the slam on the floor) runs program 65, pppRandDownFV, which wasn't in `kh2dpx::KH1_SAME`, so `start_el` refused it ("runs programs [65], not in KH1 as KH2's yet"). Every other program of Valor's move elements was already in (Checked (files), all of `P_EX100_BTLF.a.us`: only the arrival's 5 and 6 still need programs, the KeTh family, not this move). KH1's table entry 65 (`0x14054FF00` + 65 x 0x50: calc `0x14032D1C0` only) against KH2's (`0x140779A00` + 65 x 0x50: calc `0x1404C2840` only), Checked (exe) for the tables and Checked (decomp) for the code: the same decompiles but for the paused byte's offset and the random float and zero vector each game calls, the same differences as RandFV (63), which is already in. So 65 joins `KH1_SAME`. The slam's launch is its hitboxes, not the effect: ATKP 197 / 198 (reaction 11, a blow, knock-back 60 / 5) on both Keyblades' group 3 (three spheres of 140 along 100..380) at 44..65 (Checked (files)), KH1's airborne knock-back since t741; not traced. Not checked in game.

### 2026-10-27: Stage 759 (testing branch: a row's hop isn't held by the move it cuts)
- **A row's hop isn't held by the move it cuts.** The tester (drive-forms, after m2): Valor's last air finisher still drops before its hitboxes, "I don't think the problem with the last finisher is because of how you coded valor but more because of sora' main physics/animation slots ... please look outside of valor"; the owner: the air-physics thread "has a similar problem". Their trace f14640 (Checked (trace)): row 7's hop (14, Valor's 184 out of 183's Finishing Plus window) is written at f14526 in KH1's control step (`combat::exec` -> `shared::physics::kh2_hop`), but KH1's slot still plays 183 there (its frame 48, in its no-gravity range 16..end), so that frame's air step held him (no rise) and set the hold flag again after t739 had cleared it; at f14527 184's frame 0 has no hold, so the hold's end restarted his fall from rest (`kh2_fall_at`, KH2's `0x1403B4780`, t653) and the hop was lost: 52 down by 184's own hold at 16. Not Valor's: any row whose hop starts while a held move plays (base Sora's air finishers out of a Finishing Plus window, a hop out of an air cast's hover) went the same way. KH2 starts the row's motion and its hop in the same think (`0x1403D78D0` -> `0x1403CBB00`, Checked (decomp)), so the old motion's range never holds it. `HOP_FRAME`: on the frame a hop starts, KH1's air step holds nothing for the move being cut (its no-gravity range, KH1's cast hover); the rising rows' own bridge (`kh2slots::rise_start`, Finishing Leap's hop of 0) and the Drive's arrival hold as before. t739's flag clear is kept. Not checked in game.

### 2026-10-27: Stage 758 (testing branch: Valor's ground finisher lifts off from the control step)
- **Valor's ground finisher leaves the ground from KH1's control step.** The tester (drive-forms, after m2): "valors last ground finisher still doesn't bring you off the ground, it seems like a similar issue to how we fixed sora's square moves that send him off the ground". KH1's think (Steam `0x1402A3300`, Checked (decomp)) runs the control step (`0x1402A6220`), then the state's step (the table at `0x142DB3858`), then the physics. t740 changed him to the air state (`0x1402B1960`) inside the ground state's step, so KH1's air step (ours, `shared::physics::air_step`: the no-gravity hold and the root's rise) didn't run that frame and nothing lifted him before KH1's physics: **Assumption** that KH1's physics then set him down on the floor and played its landing (no trace of t740). Base Sora's Finishing Leap (t675) enters the air state from the control step (KH1's row runner, `combat::exec`), and KH1's air step lifts him by the root that same frame. Now `shared::physics::form_lift_off` runs after KH1's control step (`takeover::shared_control`), with t740's test (`kh2slots::lifts_off`: a form's slot in its no-gravity range with its root rising; Valor's 154 from frame 64, about 3.5 up a frame, Checked (files)); the ground state's step no longer does it. Not checked in game.

### 2026-10-27: Stage 757 (testing branch: a Drive Form's magic ignores the Magic Combo row)
- **A Drive Form's magic always casts by KH2's rules.** The tester (drive-forms, after m2): "drive forms magic shouldn't be affected by the magic combo setting". Wisdom on KH1's own states followed the Magic Combo row for KH1's Gravity, Stop and Aero (t716 / t719, the tester's earlier m16 answer 8). KH2's magic command counts every cast as a step of the combo with no setting (`0x1403D6CA0`, Checked (decomp)), so `formmagic::wisdom_combo` no longer reads the row: native Wisdom's Gravity, Stop and Aero always take KH2's Thunder / Magnet / Cure roles on the ground (`roles_on`) and a step of the air combo in the air (`kh1_air_cast_step`). Wisdom's own KH2 spells (Fire .. Cure, Reflect) never read the row. The row covers base Sora only, as its help line and the ini say. Not checked in game.

### 2026-10-27: Stage 756 (testing branch: L2 + R3 confirms in the command menu)
- **L2 + R3 is KH2's confirm.** The user's go-ahead on the command menu plan (`docs/kh1-kh2-command-menu.md`). KH2's field button table (Steam KH2 `0x1405C34F0`, Checked (exe)) turns raw L2 + R3 into logical bit 33, and the menu step reads `0x200000004` as confirm (`0x1403B11E0`, Checked (decomp)). With the L2 Stick Menu row on: `camera::l2_r3` sees R3 held with L2 on a pad record (`probe`), takes R3 out of that record (KH2 gives R3 no other use with L2 held, so KH1's camera reset doesn't see it), and `cmdmenu::pre_step` turns a fresh press into KH1's confirm bits (the button swap's mask `0x1405456A0`) on KH1's battle buttons before its command step. The row's help line says so.
- Not checked in game. Assumption: raw R3 is `0x4` in KH1's pad records (the PS2 order: Select 1, L3 2, R3 4, Start 8), as KH1's other raw bits in `camera` are PS2's.

### 2026-10-27: Stage 755 (testing branch: Command Lock, KH2's rule natively)
- **Command Lock row (KH1 | KH2, `command_lock`, KH2 the default).** KH2 takes a chosen command only when Sora can act on it (Steam KH2 `0x1403B6950` / `0x1403BA070`: no action, or the action's window takes a command); before that confirm does nothing, no sound, the list stays open, nothing waits. Its rows grey for availability only (MP, items, form; `0x140401D60`), never for what Sora is doing (Checked (decomp)). KH1 greys and blocks spells and items by the movement state's bits 0x20 / 0x40 (`0x1402944C0`, `0x140294220`) and takes any unblocked command, which then runs or is dropped by its control step (Checked (decomp)).
- KH2 natively (`cmdmenu`): KH1's top fill (`0x140294220`, hooked, Steam) runs with the state's "may cast, may use items" bits set in the battle states (0, 2, 6, 8, 0x18, the states KH1 casts in), put back at once, so KH1's own availability (MP, the items held, range, the save's flags) is all that greys; KH1's own states (ladders, swimming...) keep KH1's rule. KH1's run (`combat::run_entry`) refuses a spell, item or summon (a shortcut's too) while `cmdmenu::locked`: KH1's can-act (`0x140297870`) and KH1's combo window (`+0x1CC` 0x400000, which our slots carry from KH2's combo and Finishing Plus windows) both closed, as `form`'s Drive rule; while KH2's controller has Sora (the fallback), its own `can_take_command`. Attack is left to KH1's combo input; a list opening isn't a press.
- Gutted: `spell_not_yet`'s KH2-controller sections (items and summons waiting on `can_take_command`, spells on `can_cast`, the air refusal of items while KH2 had him). Kept: the Drive's own rules, Valor's no-magic, the KH1-cut-window Attack and Drive.
- Correction to the research (`docs/kh1-kh2-command-menu.md`): KH1 doesn't close an open list while Sora is busy; it greys and blocks the list's entries (the list closes only when its top row is blocked, which the state doesn't do).
- Assumption: KH1's can-act covers KH2's free states, as `form` assumes. Not checked in game.

### 2026-10-27: Stage 754 (testing branch: Cursor Memory, KH2's cursor rules)
- **Cursor Memory row (KH1 | KH2, `cursor_memory`, KH2 the default).** Steam KH2, Checked (decomp): once a command runs every open list's cursor and the top cursor go to row 0 (`0x1403B21D0`); a list's cursor is kept by command id, so a rebuilt list keeps it on the same command, and a command gone leaves the row (`0x1403B11E0` / `0x1403B1140`); cancel resets the top cursor only when it lands on the top level. KH1 (Steam, Checked (decomp)): after a command only the top cursor goes to 0 (not for commands with its "run on held" flag, `0x1402937F0`), lists keep their row (`0x140293480`), the cursor is a row number clamped to the list, and cancel resets the parent's cursor at any depth (`0x1402931B0`). Both keep a list's cursor on a plain back-out and reset the top cursor when L1 is let go.
- KH2 natively (`cmdmenu`): `combat::run_entry` notes the open lists before KH1's run and, when it returns 1, puts every one and the top level on row 0 (`command_taken`); the command under each list's cursor (magic, items, summons / forms) is noted after KH1's step and, after KH1's next top fill, a cursor whose row now holds another command follows it (`follow_commands`); a cancel two levels deep keeps the parent's cursor KH1 set to 0 (`pre_step` / `post_step`).
- Not checked in game.

### 2026-10-27: Stage 753 (testing branch: KH2's page flip sound)
- **The page flip plays KH2's sound through KH1's engine.** The user: "for when kh1 doesn't have a sound that covers functionality thats in kh2 then we add another sound e.g the sound for when you flip to page 2". KH2 plays system sound 0x14 on the flip (`0x1401DD170 (0x14)` from `0x1403F2470`; system sounds are bank 2, `se/se000.win32.scd`, `0x1401DC770`; Checked (decomp)). `kh2sound` now asks for se000's sound 20 with the others (cut into a one-sound bank for KH1's engine, the sound cache's wants change once; bus 4 goes on KH1's effects bus as the others) and `kh2sound::page_flip` plays it through `kh1bank::play`. Until KH2's sounds are loaded, KH1's cursor sound. Every other menu sound stays KH1's.
- Assumption: bank 2's sound 0x14 is se000's id record 20, the numbering of KH2's effect sound ids (169 = se000's 169). se000's 20 cuts as one track of 997 ms (Checked (files)). Not heard in game yet.

### 2026-10-27: Stage 752 (testing branch: KH2's two pages, Drive on page 1)
- **The pages, KH2's layout.** Steam KH2, Checked (decomp + files): page 1 (menu 0, `0x1403F29E0`) is Attack, Magic, Items, Drive (Revert in a form); page 2 (menu 6, `0x1403F3860`) is Attack, Summon (`cmd` 0x48), Party (0x49), Limit (0x51), a row not available being the blank command 0x68; d-pad left at the top flips them, the cursor keeping its row (`0x1403F2470`, KH2's button table: raw d-pad left is logical 0x10000, Checked (exe)). Main's page 2 was KH1's menu with its Summon row turned into Drive: the reverse.
- `cmdmenu` (new): the page flip moved from `drive` (`pre_step`, KH1's d-pad left at the top level, only with D-Pad Left KH2; the cursor kept). Page 2's rows over KH1's top level after KH1's own top fill (`page2_rows`, the fill hooked at `0x140294220`, Steam: the Items row is written after the battle lists' fill `0x1402944C0`, so page 2 can't be set there): entry 1 Summon as KH1's row 4 has it (the magic list's last entry, Summon or the summon out, with its bits; KH1's empty command 0x42 with no summon), or Drive (command 6, opening menu 3 with the forms) with Drive on page 2; entry 2 Party, 0x42 blocked; row 4 is KH1's own and empty while entry 1 isn't Magic (Limit). Party and Limit later (the user: "(a) we'll add them later").
- `drive`: no page of its own; Drive's row and list on whichever page `cmdmenu::drive_shown` says. Drive on page 1 is KH1's row 4 (the magic list's hidden last entry, KH1's own mechanism for row 4, as before); on page 2 the top level's entry 1. The research plan said KH1's fill, draw and step would take a real fourth entry; KH1's row 4 already is its own engine's row (drawn by `0x1402769D0`, run by the step from the magic list's last entry), so it is kept: no change to how it plays, less of KH1 patched. Revert at once on either Drive row (`is_drive_row`). The summon-out guard on the flip is gone (a summon out keeps KH1's row 4).
- Not checked in game.

### 2026-10-27: Stage 751 (testing branch: the Command Menu rows)
- **Command Menu list: Drive Page, D-Pad Left, Cursor Memory, Command Lock, then L2 Stick Menu.** The user: "gut the system and make sure that its 1:1 and any features that are different between the two (remembering commands on sub menus, locking your command until your actionable etc) are in the command menu config", then (round 1) "a toggle where you can switch summons or drive either on page 1 or 2", (round 2) "(b) but you can add a config in the menu for vanilla kh1 behavior", and "just do all of them". Ini: `drive_page = 1 | 2`, `dpad_left`, `cursor_memory`, `command_lock` (`kh2` the default for each, Drive on page 1). Each row saves to the ini and switches in game (`cmdmenu`). With D-Pad Left KH1 there is no page 2: what the Drive Page row puts there can't be reached. Steam only (`cmdmenu::install`), as the testing branch's new hooks.
- Research and the full KH1 / KH2 comparison: `docs/kh1-kh2-command-menu.md`.

### 2026-10-27: Stage 750 (testing branch: Aero says KH2's Reflect line with Battle Quotes KH2)
- **Aero, Battle Quotes KH2: KH2's Reflect finisher line.** The user: "kh2 battle quotes still doesn't say defense on aero, it should be a line from reflect". `kh2sound::spell_line` now says KH2's Reflect finisher voice on every Aero cast (MAGC Reflect `voice_finisher` 25, VTBL clip 31 "Protect!", Checked (files)) through `reflect_voice`, which is loaded whatever Battle Quotes says (`REFLECT_VOICES`), in place of KH1's clip 46 (t749, Assumption that it was "Defense!"; not heard in game). Why t749's line wasn't heard isn't known (no log): the hook point is KH1's command-step call (return `0x1402A3666`), which Aero's command reaches with its voice bit set (`combat::command` clears it only for KH2's native spells, Checked (code)). Not run in game.

### 2026-10-27: Stage 749 (testing branch: Aero keeps its line on every cast)
- **Aero is out of the grunt rule, as Cure.** The user: "Aero should still say its battle quote with kh2 settings because it isn't part of the combo row like cure". Battle Quotes KH1: `kh2sound::kh1_spell_group` now covers Gravity and Stop only, so Aero's cast keeps KH1's own group 13 on every cast (t747 gave it a grunt before its finisher). Battle Quotes KH2: `kh2sound::spell_line` says KH1's "Defense!" on every Aero cast (t746 had it on the finisher only, voice 7 before), as KH2's Cure says its line on every cast. Not run in game.

### 2026-10-27: Stage 748 (testing branch: KH2's item voice on KH1's item motion)
- **Battle Quotes KH2, Misc Animations KH1: KH2's item voice.** The user: "kh2 sora is missing item throw sound". KH2's item use (motion 9) says voice 3 at frame 0 (Sora's clips 23 / 24, the -3 choice a partner's line; Checked (files), `P_EX100.mset` and VTBL); our KH2 item slot carries it as a key, but KH1's own item motion (slots 58 / 59) has only KH1's item sound (11003 / 11004 at 30.8, Checked (files)), so with KH1's animation nothing said it. `combat::command` now says it (`kh2sound::item_voice`) as KH1's item command starts, when the item plays KH1's motion (Misc Animations KH1, or no KH2 item slot). Assumption: that "item throw sound" means this voice (the log had the Sound category off). Not run in game.

### 2026-10-27: Stage 747 (testing branch: KH1's Gravity, Stop and Aero grunt before the finisher at Battle Quotes KH1)
- **KH1's spells on KH2's cast animation follow t744's rule.** The user: "gravity is still playing its kh1 sounds before the finisher when on kh2 magic settings". t744 covered KH2's own spells only; KH1's Gravity, Stop and Aero are KH1's command step's, which says the spell's group (`spell + 7`) on every cast (Checked (decomp)). Now, with the spell's animation row at KH2 and Battle Quotes KH1, `kh2sound::kh1_spell_group` (from `soundlog::voice_group`, KH1's command-step call only, return `0x1402A3666`) turns the group into KH1's swing grunt (4) on a cast; on the combo's finisher the spell's own line stays. Then the user: "same goes with stop on kh1 battle quotes with kh2 magic settings": Stop plays as KH2's Magnet in KH2's combo, which has no finisher cast, so the finisher is the combo's last step as noted for the cast (`formmagic::cast_last`, `note_step`'s `CAST_STEP`, read before `take_step` takes it), else KH2's finisher cast picked (`kh2slots::finisher_picked`); the same for t746's Battle Quotes KH2 lines (`kh2sound::spell_line`). Not run in game.

### 2026-10-27: Stage 746 (testing branch: KH2's spell lines with Battle Quotes KH2)
- **KH2's magic voices loaded, and KH1's spells speak KH2's way.** The user: "kh2 sora is missing his finisher grunts on magic, for aero can you give him the line where he says defense", then "for magnet just do a generic finisher line he does for his base finisher swing". Cause of the missing finisher lines: KH2's cast code plays the MAGC voices (cast 7; finisher Fire 11, Blizzard 12, Thunder 13; Cure 22 / 23; Magnet 24, Checked (files) in `00battle.bin`), not a motion trigger, and `kh2sound` loaded only motion voices and Reflect's, so `formmagic::kh1_spell_start` asked for 11 / 12 / 13 and nothing played. Now every spell's MAGC voices load. KH1's own spells (KH1 rows, Gravity, Stop, Aero), whose line KH1's command step plays (`0x1402A3600`: group `spell + 7` through `0x1402901F0`, return `0x1402A3666`, Checked (decomp + exe)), are said KH2's way with Battle Quotes KH2 (`kh2sound::spell_line`, from `soundlog::voice_group`): voice 7 on a cast; on the combo's finisher (its KH2 finisher cast picked for the ground request, `kh2slots::finisher_picked`; none in the air) Fire 11, Blizzard 12, Thunder 13, Gravity and Stop KH2's base finisher swing line (voice 5, the user's "generic finisher line"), Aero KH1's "Defense!" (`kh1voice::defense`, clip 46, Assumption that it is "Defense!"); Cure KH2's self-cure line 23 on every cast. A voice not loaded leaves KH1's. Not run in game. Docs: `docs/kh1-voice.md` "Spell voices".

### 2026-10-27: Stage 745 (testing branch: KH2's Drive line with Battle Quotes KH2)
- **Drive: KH1's "Give me strength!" only with Battle Quotes KH1.** The user: "hes also missing his drive transformation lines". `form.rs` played KH1's clip 37 at priority 3 on every Drive (both start paths); KH2's own line, voice 8 on the Drive motion 52 at frame 0 (VTBL clips 25 / 32 / 33, priority 2, Checked (files)), fired a frame later (seen in the user's t743 log) and KH1's voice rule kept it out (Assumption: heard as missing, not logged). `form::drive_voice`: KH1's line only with Battle Quotes KH1. Not run in game.

### 2026-10-27: Stage 744 (testing branch: KH2 magic with Battle Quotes KH1 grunts, then the spell line on the finisher)
- **KH2's spells at Battle Quotes KH1 follow KH2's voice rule with KH1's lines.** The user: "for when kh1 sora does any kh2 magic we need to make him grunt like kh2 sora until he gets to his finisher, then he plays one of his normal magic lines". KH2's rule (MAGC: voice 7, a grunt, on every cast; the spell's line on the finisher cast, Checked (files)). Before, KH1's command step played KH1's spell group (`spell + 7`, every cast, Checked (decomp)). Now, for a native KH2 Fire, Blizzard or Thunder, the command's voice bit is cleared at either Battle Quotes (`combat::command`) and `formmagic::kh1_cast_voice` plays KH1's swing grunt (group 4) on a cast and KH1's spell group on the combo's finisher, through KH1's own group play; the same on the Drive Forms' KH2 casts (`before_step`). Cure, which KH2 casts with no finisher, keeps KH1's own line on every cast. Not run in game.

### 2026-10-27: Stage 743 (testing branch: KH2's quotes on KH1's own moves)
- **Battle Quotes KH2 on a move that plays KH1's own animation.** The user: "keep working on mixing kh1's animations with kh2s voices and vice versa". KH1's attack row start says Sora's row voice (the row's `+0xF`, voice group play `0x1402901F0`, Checked (decomp)); with Battle Quotes KH2 that grunt was left out only on rows playing a KH2 motion (`kh2sound::holds_back`), so a move at the KH1 style still said KH1's grunt. Now the same hook says KH2's voice of the same kind instead: KH1 group 4 (combo swings) -> KH2 voice 4, 5 (finishers) -> 5, 6 (air finishers) -> 6 (`KH1_TO_KH2`), at the row's start, where KH2's swing quote is too (frame 0 of KH2's swings). Checked (files): KH1's groups by the rows that call them (`btltbl.bin`, `docs/kh1-voice.md`), KH2's voice triggers in `P_EX100.mset` (151..153 voice 4, 154 / 155 voice 5, 165..167 voice 6). A group with no KH2 counterpart (e.g. 19, motion 212's row) and a voice that isn't loaded stay KH1's. `holds_back` takes the group now (`soundlog::voice_group`). Not run in game. Docs: `docs/kh1-voice.md` "Mixing".

### 2026-10-27: Stage 742 (testing branch: KH1's grunt on KH2's dodge roll with Battle Quotes KH1)
- **KH2's dodge roll, Battle Quotes KH1: a swing grunt again.** The user, after the voice research ("KH1's roll is silent", `docs/kh1-voice.md`): "okay add it". The native roll (`combomods::roll_press`, KH1's roll with KH2's motion 209 baked in its slot) no longer passed through the KH2 controller's DodgeRoll state, which called `kh1voice::dodge_roll`. KH2's roll has its own voice (frame trigger 14, voice 7 at frame 0, Checked (files)), baked as a `KH2_VOICE` key; with Battle Quotes KH1 `kh2slots::sound_event` now hands that key to `kh1voice::stand_in`, which plays KH1's group 4 (KH1 picks one of its five clips) for the moves in `STAND_INS` (only the roll: a move with an attack row already has KH1's row voice). Fires on KH1's motion pass at the roll's start, no per-frame code, no rebake (the key was already in the slot). Battle Quotes KH2: KH2's voice 7 as before. Quick Run (KH2 voice 36 at frame 0, Checked (files)) is left silent at Battle Quotes KH1, as asked before. Not run in game.

### 2026-10-27: Stage 741 (testing branch: every KH2 blow is KH1's airborne knock-back, no height cutoff)
- **One KH1 reaction for KH2's one blow reaction.** The user, reviewing t721: go ahead with "send every KH2 reaction-4 hit through one KH1 reaction, with no height cutoff ... remove the Small Blows toggle". KH2 plays blow motion 16 / 18 and its landing for every group-4 hit (Steam KH2 `0x1403D2FE0` case 4, Checked (decomp)); t721 split it at our own 25-unit arc and dropped Fire's lift on the ground. `combat::scale_record`: every KH2 blow on an enemy of actor kind 3 (byte `+0x6`; enemy think `0x1402B7260`) is KH1's kind 4; scripted kinds 4 / 5 / 9 / 10 (object think `0x1402C2C90`, picker `0x140297B5A`) and unknown targets keep kind 5 (`script_launched` as before). `combat::knocked_back` (hook on `0x1402B77D0`): for a KH2 blow's record, a grounded enemy KH1 would answer with its ground branch (not lying, `0x1402B1AE0` false, `0x1402B1B10` false) that has motion 0x50 (`0x14029F010`) is lifted with KH1's own hop `0x1402AF960` (20) so KH1's knock-back takes its airborne branch itself (0x50 / 0x51, no lying down: the think lands a non-launch motion on `+0x3D0` + 7); then KH2's arc and sideways speed as t721. The airborne branch is told by motion 0x50 / 0x51 first in the motion queue (`+0x17C`, written by `0x14029F980`), not the hop's float. State 2's class in the table `0x142DB3840` checked (not 2); the four functions' first bytes checked once. All Checked (exe). Small Blows row, its texts (`0x7F000223` / `0x7F000224`), `knockback::small_blows` and the ini key removed (Tweaks has 12 rows). Assumptions: KH1's 0x50 / 0x51 looks like KH2's 16 / 18 and holds through a long arc; KH2's blow has no down time; how each enemy's `+0x3D0` + 7 landing looks. Not checked in game. docs/kh1-launch.md §7.

### 2026-10-27: Stage 740 (testing branch: Valor's ground finisher goes up)
- **Valor's ground finisher goes up.** The tester (drive-forms): "final finisher on ground doesn't put you in the air". Checked (files): 154 has no gravity from 63 and its root rises 0 -> 125 over 64..116. `shared::physics::lift_off` puts him in KH1's air state 2 (`0x1402AF7D0` via the map) when a form's ground move rises by its root in its no-gravity range: motion kept, arc from rest, the queued idle becomes KH1's fall; with baked travel only the root's height is added in the air. Assumption: state 2 runs the swing to its end. Not checked in game.

### 2026-10-27: Stage 739 (testing branch: Valor's air finisher keeps its hop)
- **Valor's air finisher keeps its hop.** The tester (drive-forms): "you drop a little bit before the hitboxes come out on final air finisher". Trace f14640: 184 started with no rise and fell ~50 before its hitboxes. 183's hold-end fall restart (KH2 `0x1403B4780`) measured the held frame and replaced 184's row hop of 14; a fresh hop is no longer restarted (`kh2_hop_as` clears the hold-last flag). Assumption: KH2's speed there is the hop's start speed (`+0xA3C` not traced). Not checked in game.

### 2026-10-27: Stage 738 (testing branch: Form air moves steer and take their root)
- **Form air attacks and casts steer and take their root.** The tester (drive-forms): "air fires physics are wrong", "all the air magics movement and physics aren't proper", "air attacks movements seem off". Trace f46520: 181 dragged with the stick held. In a form action's allow-movement range (id 41) KH1's air step steers as in a jump, and the clip's root is added on every air frame (`air_root_form`; 219 of air Fire 58's 383 come before its no-gravity range 16..36). Source: the OG rewrite's action step (Assumption: not re-traced in the Steam KH2 decomp). Base Sora unchanged. Not checked in game.

### 2026-10-27: Stage 737 (testing branch: Wisdom's Fire dashes)
- **Wisdom's Fire carries him.** The tester (drive-forms): "fires physics are missing". Checked (files): Wisdom's 56 / 57 / 58 travel 384 / 836 / 383 (base Sora's casts none). A form's ground cast with root travel is baked with it and its request gets KH1's travel bit 15 in `redirect`. Not checked in game.

### 2026-10-27: Stage 736 (testing branch: Wisdom moves while attacking)
- **Wisdom moves while attacking on the ground.** The tester (drive-forms): "you can't move and attack on the ground". Trace f45615: no movement through 151 with the stick held. Our combo rows stand for KH1's swing slots 98..101, and that branch of `ground_step` skipped `ground_inertia`; it now runs it, in each swing's allow-movement range (151: 0..46, 154: 10..80; Checked (files)). Not checked in game.

### 2026-10-27: Stage 735 (testing branch: Wisdom's Reflect on KH1's cast state)
- **Wisdom's Reflect on KH1's cast state.** The tester (drive-forms): "reflect is playing the base animation", "air reflect is base soras reflect". Log f26207: KH2's deprecated controller borrowed him and drew base 71. `reflect::native_path` now covers native Wisdom: its own 71 / 72 / 73 (Checked (files), MAGF) on our Reflect rows, sub +10, KH2's cost through mpcost. Not checked in game.

### 2026-10-27: Stage 734 (testing branch: A Drive Form's own Keyblade hold)
- **A Drive Form's own Keyblade hold.** The tester (drive-forms): "sora doesn't spin his keyblade on the idle animation". Checked (files): `W_EX010_MAGF` motion 0's root turns 75..246 degrees from base Sora's hold (base Sora's within 8; Valor's about 95). `pose::form_grip`: form bakes turn bone 280's subtree by the form's root turn, in model axes through KH2's hand 178; form slot keys salted (one rebake). Assumption: KH1's rest grip stands for base Sora's KH2 hold. Not checked in game.

### 2026-10-27: Stage 733 (testing branch: Gravity's finisher back)
- **Gravity's finisher back.** The tester (drive-forms): "gravity doesn't have a finisher". Trace f23582 and the log: 62 on every cast. t719's ground-role pick in `combat::command` was dropped by t721's commit (made from an older combat.rs); restored. Checked (code). Not checked in game.

### 2026-10-27: Stage 732 (testing branch: Wisdom's Square is its Quick Run)
- **Wisdom's Square is its Quick Run.** The tester (drive-forms): "you dodge roll instead of quick run". Trace f17582: KH1's own roll (0xDC). `quickrun::built()` refused every Drive Form (a t501 rule), so KH1's menu step (Steam `0x1402937F0`, pressed `0x10` -> command `0x4000000`) led to KH1's roll in `0x1402A7B30` (ability bit 0x20 of `0x142D5EC10`, set by `0x1402A6380`) (Checked (decomp)). Native Wisdom now takes KH1's Quick Run state. Not checked in game.

### 2026-10-27: Stage 731 (testing branch: Chest Opening: the notice at the top left)
- **The notice sits at the top left.** Mr Bugs: "1. (b)" (the notice at the top like KH2's info bar). KH1's box updater `0x140272F10` (boxes `0x14283B390`) stacks columns from 0x10 below the top, from the left (Checked (exe)), where KH2's info bar sits; the row's help text says so. Not built: KH2's own info-bar art and timing (needs a KH2 2D layout reader on testing; the give path `trap_treasure_get` `0x1404317F0` -> `0x1401595B0` not traced).

### 2026-10-27: Stage 730 (testing branch: Chest Opening KH2: gummi and Dalmatian chests)
- **Gummi and Dalmatian chests take the KH2 path.** Mr Bugs: "the other parts of the code played except for chests that have gummi's in it and Dalmatians". Chest table `0x140528D60` (Checked (exe)): kind 1 gummi blocks (given by the Open handler through `0x14020D3F0`; full stays KH1's), kind 2 the Dalmatians (33 entries; the chest script gives them, message function 605 only builds the window line, Checked (decomp)). Both now take the KH2 path; notice "Gummi Block" / "Dalmatians NN, NN, NN". Assumptions: kind 2 = Dalmatians from the count; set n = puppies 3n-2..3n; the gummi's own name isn't read. Not checked in game.

### 2026-10-27: Stage 729 (testing branch: Chest Opening KH2: no open motion, as KH2)
- **No open motion on Sora, as KH2.** Mr Bugs: "3. (b)" and "sora still doesn't play his kh2 chest opening animation". KH2's Open (03system cmd 0x1A5: execute 421, action 0; Checked (files)) is a message to the box, which plays its own lid motion 0x98 with an 8-frame blend (Steam KH2 `0x140405ED0` / `0x140406020`, set-up `0x140402200`; Checked (decomp)); KH2's player has no chest action (Checked (exe RTTI)), so KH2 Sora plays no chest motion. KH1's pending open request 0x900A6 (`+0x178` / `+0x17C`) is replaced by Sora's current motion (`+0x164`) through `0x14029F980` (actor, request, f32 in XMM2; Checked (exe)). Assumption: re-asking for the playing motion doesn't restart it. Not checked in game.

### 2026-10-27: Stage 728 (testing branch: Chest Opening KH2: the treasure script no longer holds Sora)
- **The treasure script no longer holds Sora.** Mr Bugs: "wasn't able to move when opening a chest, I stayed in idle for a bit". The t725 trace: Sora's actor flags 0x1000000 / 0x200000 / 0x40 on for 132 frames after the Open (KH1 actor actions 0x24 / 0x26 / 0x2B queued by the chest script; 0x10 can relock `+0x444`; runner `0x14028E6E0`) (Checked (trace + decomp)). At a KH2 open our handler takes the call class 0x18 slot of KH1's script class table (`0x1404E5E70`, KH1's `0x1401B3530`, put back after 8 s); from treasure contexts the actions it queued on Sora (ring `0x1423CFE10`, write `0x1423CFE08`) go to KH1's done queue through KH1's own push `0x1401A4F20`, as KH1 does for a missing actor, and event function 103 (`0x1401A96F0`, wait on the context actor's motion) on Sora goes on at once (Checked (decomp + exe)). Assumption: the chest script file itself is unread (which actions it queues). Chest log lines now under Cutscenes & Events. Not checked in game. docs/kh1-chest.md.

### 2026-10-27: Stage 727 (testing branch: Chest Opening in QoL, KH2 by default)
- **Chest Opening moves to QoL, KH2 by default.** Mr Bugs (kh2-chest, round 2): "2. (b) and move it in QOL". The row leaves Combat > Tweaks for the QoL list; `chest_style` defaults to kh2 (a missing key means kh2, the default ini writes kh2; an ini that already says kh1 stays kh1 until the row is switched). No addresses.

### 2026-10-27: Stage 726 (testing branch: the Aerial Dodge's sound back)
- **The Aerial Dodge's 999019 plays again.** The user: "from the switch to native audio air dodge lost its sound". Cause: t677 keys KH2's frame triggers at F - 1/64 for KH1's motion pass (`from <= t < to`), and t645's `combomods::dodge_clock` puts the dodge slot's time at 1 before KH1's update, so the update steps 1 -> 2 and the frame-1 key (0.984) is never in range (KH1 `0x14029F0A0` reads `from` from `+0x16C`, passes it to `0x14029FB30`, Checked (decomp)). KH2 fires frames 0 and 1 in its one 0 -> 2 update (Checked (decomp), t645). `kh1_data::slot::kh2_trigger_time_skipped`: motion 202's frame 0 and 1 triggers (sounds, voices, effect elements) are keyed at exactly 1.0, so the update from 1 fires them on KH2's update; later frames unchanged (`kh2slots::kh2_trigger_keys`). The slot cache rebakes the dodge by itself (its key holds the tracks). Test `skipped_start_fires_frames_0_and_1_on_the_update_from_1`. Not checked in game; Assumption: 202's only early trigger is 999019 (not read from the files). Where the skip does not apply (a step of 1 with a blend or a time past 0) the keys fire one frame late. docs/kh1-aerial-dodge-native.md §9.

### 2026-10-27: Stage 725 (testing branch: a big body takes KH2's rewritten reaction, so Fire and the first swing only push it)
- **KH2's big-target reaction rewrite.** The user, after t721: "fire still staggers the enemy, even if the launch goes far, what I mean is how regular hits work the base combo 1st swing staggers small enemies but only the finisher can stagger bigger ones (big bodies) but right now kh2 air and normal fire can stagger". KH2 rewrites a hit's reaction for a big target with the `patn` table in 00battle.bin (rows 7 and 8; applied by Steam KH2 `0x1403EE9B0`; Checked (decomp + files)): 3 and 0x10 become 1 (push only), 4 and 0xF become 0xC (half push), 6 and 7 become 0xD (stagger, half push). Our code never applied it. `combat::scale_record` now does, for a target `Body` measured as big (its existing size measure, count > 8, tall > 600 or radius > 300, kept per actor): `knockback`'s record gets the rewritten reaction (its push distance follows), no KH2 launch or small blow is made, and the record's kind becomes KH1's push-only kind 2 (push categories) or flinch kind 3. Small enemies are untouched. Assumptions: KH1 big = our size measure (KH2's own flags are the objentry big bit / armor); a big body not yet measured (never in the combo's reach) counts as small; KH1's kind 2 plays no stagger on the big enemy (not checked in game). Not changed: other rows of the table (conditions 4, 5, 6, 12, 18 and the rest) are not applied.

### 2026-10-27: Stage 724 (testing branch: Chest Opening (Tweaks): KH1 | KH2)
- **Chest Opening (Combat > Tweaks: KH1 | KH2, ini `chest_style`, KH1 by default).** The tester Mr Bugs (kh2-chest): "quickly open and move from the chest opening animation like in kh2 ... where you open the chest, move then the pop up for the item plays at the top of the game". KH1's Open handler `0x1402B58E0 (sora, chest)` (command 0x37 from `0x1402A3DB0`; chest table `0x140528D60`) gives the contents itself before any script (item: stock add `0x140284E60 (item, 0xC)`, 0xFF when full and the chest stays shut; munny x10 `0x1402E7E10`; reward record `0x14028F9A0`), marks it opened (`0x140285800`), counts it, stores its arguments at `0x142DB7B28`, locks Sora (`+0x444` bit 1), asks for the open motion (`0x14029F980 (sora, 0x900A6, 0)`) and queues event 5, which starts `exchange/<lang>_treasure.ev` (`0x1401A43A0`, contexts `0x1423BF0B0`) for the lid and the "obtained" window (Checked (decomp + exe)). New `win/chest.rs` (Steam only; hook on `0x1402B58E0`, first 10 bytes checked; installed once the row is first set to KH2): at KH2, for a plain chest (kind 0 or 3 after KH1's handler, stock not full), KH1's handler runs whole, then the lock bit is cleared so KH1's control step runs him at once, and for 8 s the event table (`0x1404FA870`) entries of the window functions 0 / 1 / 177 / 605, when called from a treasure context, take KH1's own skip-mode branch (no window; the script's waits pass with none open); "Obtained" and the item (or munny) show in KH1's level-up notice box (`menu::queue_notice`, first free frame). Event chests, other rewards, kind 1 and a full stock stay KH1's. Tweaks row after t721's Small Blows, text ids `0x7F000225` / `0x7F000226`. Assumptions: what the treasure script does besides its window (the file wasn't readable this session), KH2's own timing (KH2's treasure code wasn't in the decomp at hand), where the box shows, the Tweaks rows clearing the help box. Not checked in game. docs/kh1-chest.md, docs/kh1-config-menu.md §6.z6.

### 2026-10-27: Stage 723 (testing branch: Superglide is KH2 Glide LV1, only Final raises Glide)
- **Superglide is KH2 Glide LV1, only Final raises Glide.** The tester (what-is-still-missing, after m18, answer 3): "(b)" (Superglide as LV1 so only Final raises Glide). KH1's Superglide (id 4) now counts as KH2's Glide LV1 or Final's form-level growth, whichever is higher, and KH2's own Glide (88, the Glide style's) no longer goes to MAX past Chernabog: LV1, raised only by Final (`kh2_glide_now`; `Effect::GlideMax` and `kh2_glide_level` removed). The mod's own mapping only, no new addresses. Not checked in game. docs/abilities.md, docs/kh1-glide-native.md.

### 2026-10-27: Stage 722 (testing branch: KH2 abilities in their own list, a KH2 row in KH1's Abilities menu)
- **KH2 abilities in their own list, a KH2 row in KH1's Abilities menu.** The tester (what-is-still-missing, after m18, answer 1): "give kh2's its own ability list right next to sora, should be titled \"KH2\"". The list is 48 bytes in KH1's config block (save sub-block 5, pointer Steam `0x142E1F4D8`) at `+0x3C0..+0x3EF` (`growth::KH2_LIST_OFF`), saved and loaded with the slot and zeroed by a new game (Checked (decomp): every access through that pointer is at `+0x74` or below; Assumption: nothing reaches it through a pointer computed elsewhere). KH1's own Abilities page runs it (Steam, Checked (decomp + exe bytes)): init `0x140311850` (page table `0x14052ADA0` entry 2), main window `0x140310EF0`, member picker `0x140310B60`, list `0x140310A90` (`0x140311C20` member, `0x1403120F0` shared), header `0x1403108E0`, help `0x140310AB0`, sort `0x140310400`, equip `0x140310650`; rows `save + 0x48E + row`, count `0x142E941D0`, per-row counts `0x142E9CBC8`. `win/kh2list.rs` wraps the eight windows' update pointers (`.data` `0x14052ADB0`, `0x1405490F0`, `0x1405490C0`, `0x140549090`, `0x140548FA0`, `0x140548F40`, `0x140549030`, `0x140549060`): for each of the page's own calls it sees a member 10 ("KH2") right after Sora, whose record arithmetic puts its list at `save + 0x4CC`; our list is copied there for the call and KH1's bytes there (item counts 0x33..0x62, which the page doesn't read) put back right after, so outside the page's calls the save is KH1's. AP left `0x1402E8660` / used `0x1402E8710` add the KH2 list's equipped AP for Sora (Abilities, Status, Equipment's accessory check); member name `0x1402E86F0`, max AP `0x1402E8560` and the picker clamp `0x1403119D0` are re-made (RIP-relative starts); the party-stats refresh `0x1402E7EC0` runs with the real party during the page. With a guest, the fifth row's count spills onto `0x142E9CBD8` (another page's pointer), saved and restored around each call (Assumption: that page sets it at its own init). Saves move once (mark `0x4B04` at `+0x48A`): every KH2 ability leaves Sora's 48 for the KH2 list, on or off as it was; new grants go there; t718's reserve is gone. On Epic or a failed check ("KH2 list: not installed") they stay in Sora's list as t718 with 2 places kept. Not checked in game. docs/abilities.md "The KH2 list", docs/kh1-config-menu.md.

### 2026-10-27: Stage 721 (testing branch: small KH2 blows play KH1's knock-back, not its launch)
- **Small KH2 blows are KH1's knock-back.** The user, after a Fire log: "currently kh2 fire does a finisher stagger on the enemy with both ground and air do you know why?", then "(a)" and "go ahead". KH2's Fire hits are blows (reaction group 4) with up 1 to 6 and 0 sideways (Checked (files), ATKP 502); since t602 every KH2 blow was KH1's launch, so each played the launch motion and lay down (Checked (trace)). Now a blow whose arc is at most 25 units (`combat::SMALL_BLOW_MAX`; an Assumption, picked so Fire's hits are under it) is written into the hit record as kind 4 (`combat::scale_record`), KH1's enemy knock-back (Steam `0x1402B77D0`, hooked: `combat::knocked_back`, prologue `40 53 56 48 83 EC 68`, Checked (exe)): motion 0x4C on the ground (nothing of KH2's to put in it), 0x50 with a hop in the air, which is given KH2's arc and sideways speed and flown as KH2's blow (`knockback::fly_blow`). An enemy with no 0x4C motion falls back to its launch, with KH2's numbers in it as before. Config > Combat > Tweaks > Small Blows (ini `small_blows`, on by default); off is t602. Blows over 25 keep KH1's launch. Not checked in game. docs/kh1-launch.md §5.

### 2026-10-27: Stage 720 (testing branch: Magic Combo KH2: an air cast in a Finishing Plus window counts as a finisher)
- **An air cast in an air finisher's Finishing Plus window counts as a finisher.** The tester (what-is-still-missing, after m17, question 7): "add it". Steam KH2's `0x1403D6CA0` adds one to the finisher count `+0xD75` when the magic command is taken in a Finishing Plus window, on the ground or in the air (Checked (decomp)). `formmagic::take_step` now notes such an air cast (KH1's count already at the air length) as the combo's next finisher (`combomods::cast_step` chained: `FINISHERS` one more), for every path that takes an air step: KH2's spells on KH1's cast state, Reflect, KH1's own air casts, Wisdom's. It still plays its plain air cast (KH2 has no air finisher cast), and its count goes to 0 as before. Not checked in game. docs/kh1-magic-native.md §4c.3.2.

### 2026-10-27: Stage 719 (testing branch: Magic Combo KH2: Gravity as Thunder, Stop as Magnet, Aero as Cure)
- **Gravity as Thunder, Stop as Magnet, Aero as Cure.** The tester (what-is-still-missing, after m17): "make soras base and any form that can do drive form magic this: aero is the same as kh2 cure, gravity is the same as thunder, stop is a finisher like magnet (not in the game yet)". Steam KH2 has no combo rule per spell (Checked (decomp)): the cast start `0x1403A9DD0` asks `0x1403D7780` whether the count has reached the length, and `0x1403C5170` plays MAGC's finisher motion there if the row has one, else the ground motion (always the air motion in the air). MAGC and `P_EX100.mset` / `P_EX100_MAGF.mset` (Checked (files)): Thunder 62 (combo window from 42) / 63 (Finishing Plus window 42..78); Cure 65 (Thunder's animation with 62's ranges, which name 62: no window); Magnet 68 (63's animation, only its Finishing Plus window); air 64 / 67 / 70 one clip with no window. With the Magic Combo row at KH2, base Sora's and Wisdom's KH1 Gravity, Stop and Aero on the ground (`formmagic::Role`, `ground_role`) take a plain step at the command (`note_step`), and our slot plays Thunder's 62 (63 at the length) / Magnet's 68 / Cure's 65 (`kh2slots::CASTS`, 63 in `CAST_FINISHERS`, picked in `combat::command`), windows from the clip. On KH1's own animation Gravity and Aero get KH1's Thunder / Cure window (Fire's share), Stop Magnet's window by share of 68 (Assumption). Stop at the length counts as a finisher; taken mid-combo, Magnet's window spends one Finishing Plus (`combomods::SPENT`, cleared by a new combo: KH2's `+0xD75`). `kh1_finisher_window` no longer opens over casts. In the air all three stay a plain air step (t716). At Magic Combo KH1 nothing changes (63, the combo's finisher). Valor and the deprecated forms untouched. The moved slots rebake once. Not checked in game. docs/kh1-magic-native.md §4c.3.3.

### 2026-10-27: Stage 718 (testing branch: Vanilla ability progression, every KH2 ability given, no extra AP)
- **Vanilla ability progression, every KH2 ability given, no extra AP.** The tester (what-is-still-missing, after m16 answer 6 (b), then after m17): "make it all vanilla, just give the player all the custom abilities we added, existing saves get updated, remove extra ap for now". KH1 gives its own abilities on its own schedule: out are the mod's level schedule and boss gifts (`BOSS_GIFTS`, `MenuAbility::level`), the growth level tables, `make_room` (KH1 abilities pushed out for ours), `one_only` (spare High Jump / Glide / Superglide), Aerial Dodge taken back until Genie Jafar, and a form level putting KH1's High Jump or Glide in the shared list. All 22 KH2 abilities in Sora's own list are his, off until equipped (`Kh1Sora::grants`; three or more at once share one "New Abilities" box); they always leave `growth::RESERVE` (2) places for KH1's own appends and give theirs back (`yield_one`, ours only, never a KH1 ability) when KH1's own fill the list, coming back when there's room (Assumption: KH1 gives at most 2 abilities within the once-a-second check, and appends at the first empty byte). Growth comes only from Drive Form levels (KH2's `0x1403E8380`, t703): an ability he has is LV1; Valor / Wisdom / Final raise High Jump / Quick Run / Glide. KH1's High Jump and Glide count as LV1 once KH1 gives them (Assumption: m17 question 3 unanswered); Dodge Roll and Aerial Dodge stay LV1 (Limit / Master aren't native); Superglide stays KH2 Glide MAX. Saves are updated once (mark `0x4B03` in the u16 at save `+0x48A`, no new bytes): KH1 abilities noted in `kh1-kh2-removed.txt` up to the save's level come back off, Superglide after Chernabog; nothing writes that file any more. The bonus AP and t715's form-reward AP are removed (ini `bonus_ap` gone). The fmlv rewards (Combo Plus, Air Combo Plus, MP Rage, MP Haste) stay as KH2's own rule. Checked (code: no level-up / grant hooks remain); the decomp couldn't be reached this round (no new address used). Not checked in game. docs/abilities.md, docs/kh1-port.md, docs/kh1-drive-forms-native.md §9.

### 2026-10-27: Stage 717 (testing branch: In-game Scene Skip: Trinities)
- **In-game Scene Skip: Trinities.** The tester (auto-cutscene-skipper, after m31): "currently works in traverse town, wasn't able to test it in other worlds, but can you also make it skip trinity animations?", then "talking to phil after the barrel mini game still plays the in game cutscene where he gives you thunder" (with a log). A Trinity mark is a context of a script (the room's, entry 0: contexts `0x142382AD0`, 0x360 each, count `0x14237F4E4`; or an event entry's, `0x1423902D0`) that offers Triangle command 0x53 through event function 454 (`0x1401AF530`: room object table `0x142382938`, command +0xC, offered by `0x1401A4D00`) and, when used, starts event entry 50 through fn 126 (`0x1401A9D90` → `0x1401A38A0`), whose first three threads put Sora, Donald and Goofy round the mark. Checked (exe + decomp + the user's own room scripts: 17 rooms in six worlds; read locally, nothing in the repo). t712 missed them: some story versions of Traverse Town's districts run the Trinity without event mode (no fn 513), and a talk earlier in the room leaves the talk bit 0x800 on (the log's f32384, Traverse Town entry 50). `ingameskip` now reads the room script's contexts once per new entry, and each event entry's contexts when it starts (a mark inside an entry, as Phil's after-barrel scene's ctx 16; noted in `NEXT_TRIN`, up to 4): an entry such a mark starts is judged at once, regardless of event mode or the talk bit, and skipped with KH1's own pause skip (bit 0 + flag byte, as t712) unless it has the pause-skip function (Olympus he02: the Cutscene Skip row's) or a choice, shop or menu. Fns 454 / 126 are checked in the event table (`0x1404FA870`). New pure `kh1script.rs` (`Call`, unit-tested). Phil's Thunder scene: Triangle on Phil (world 0xb room 1) starts entry 1 with the talk bit on, and that entry also holds a choice (fn 88, var 0x910) and the Coliseum's mark: no clean line between it and a talk that must play, so talks are unchanged (asked). Assumptions: each Trinity type's result is set by the script steps skip mode runs, Sora ends where the played Trinity leaves him, 0x53 is only the Trinity command. Not run in game. docs/kh1-ingame-skip.md "Trinities", "Talks that lead into a scene".

### 2026-10-27: Stage 716 (testing branch: Magic Combo: every kind of magic takes its step)
- **Magic Combo: every kind of magic takes its step.** The tester (what-is-still-missing, after m16, answer 8): "if kh2's own air magic system takes a combo step then yes do it for all forms of magic when the magic combo setting is switched on". Steam KH2 (Checked (decomp)): the magic command `0x1403D6FC0` always asks `0x1403D6CA0 (p, 1)`, which has no ground / air test (`+0xD74` + 1 in a combo window, `+0xD75` + 1 too in a Finishing Plus window, `0x101` when free), and the cast start `0x1403A9DD0` → `0x1403D7780` measures the count against the air length `[+0x5C0]+0x201` while airborne (`0x1403BA9A0`): an air cast is a step of the air combo, the last one at its length. With the Magic Combo row at KH2: base Sora's KH1 air casts (Fire .. Aero by KH1's script, air state 2 / 8) take their step (`note_step` (air) at the command against KH1's air length, stats `+0xD5`; `take_step` (air) at KH1's spell start for requests 0x83..0x8A; `formmagic::kh1_air_cast_step`; the air combo's last step sets KH1's count `+0x41` to 0; no combo window added, KH2's air casts have none, Checked (files)); Wisdom's Gravity, Stop and Aero on KH1's states (`wisdom_combo`) count as the combo's finisher on the ground as base Sora's (§4c.4; their Finishing Plus window by share of Wisdom's clip 63, Assumption that it has a finisher range) and take an air step in the air. At KH1 nothing changes; Valor, and Master / Final / Limit / Anti (the deprecated controller), untouched. Assumption: KH1's spell start `0x14028F140` runs for air casts with the air request at the head of the queue. Known difference (since t486): an air cast in an air finisher's Finishing Plus window isn't counted as a finisher (KH2 adds 1 to `+0xD75`). Not checked in game. docs/kh1-magic-native.md §4c.3.2.

### 2026-10-27: Stage 715 (testing branch: Form level abilities add their AP)
- **Form level abilities add their AP.** The tester (what-is-still-missing, after m16, answer 7): "(a)" (the abilities forms give add their AP to the bonus AP, like the mod's other gifts). KH1's party stats hook (`menu::stats`, `0x1402A2FA0`'s max AP `+0xE4`) now also adds, for each ability a form's level has given Sora (`formexp::rewards_given`: fmlv rows up to the form's rewarded level, through `growth::kh1_reward`: Combo Plus, Air Combo Plus, MP Rage, MP Haste) that he still has a copy of in his list, KH1's own AP for it (byte +0 of its entry in KH1's ability table, `*0x142D37748` + (id - 1) * 12, Checked (decomp) as t36's notes), capped at KH1's 100 with the rest; logged with the AP line. `abilities = kh1` and `bonus_ap` on only, as the bonus. Assumption: KH1's AP (not KH2's) is the right figure, as these are KH1's own abilities. Not checked in game. docs/abilities.md.

### 2026-10-27: Stage 714 (testing branch: Wisdom LV6 gives KH1's MP Haste)
- **Wisdom LV6 gives KH1's MP Haste.** The tester (what-is-still-missing, after m16, answer 4): "(a) and when we implement kh2's mp system we turn it into mp charge when the player toggles ... the kh2 mp system in the config". KH2's fmlv gives MP Haste (KH2 id 413) at Wisdom LV6 (Checked (files), t703); `growth::kh1_reward` now maps it to KH1's own MP Haste (id 23), given as t703's other rewards: off, in Sora's list, with a "New Ability" box. KH1's MP Haste keeps KH1's own effect (KH2's shortens MP Charge, which KH1 doesn't have yet); once a KH2 MP system is a Config setting, it becomes MP Charge's MP Haste there (noted). A save whose Wisdom already passed LV6 before this version had it marked as given with no counterpart and doesn't get it. Assumption: KH1's MP Haste stands for KH2's. Not checked in game. docs/abilities.md.

### 2026-10-27: Stage 713 (testing branch: Left Keyblade: its holder blended by KH1's own motion blend)
- **Left Keyblade: its holder blended by KH1's own motion blend.** The tester (what-is-still-missing, after m16, answer 2): "looks good but I wanna see how it looks with (b)" (blend the holder through KH1's own blend pass). KH1's actor pose step `0x1402A0350` starts a motion blend with `0x140394080`, or `0x140393B30` while one runs (`(world, instance, time f32, motion)`: the old pose kept, the new motion posed into blend buffers on KH1's heap); each frame `0x1402A0620` runs the blend pass `0x140392560 (world, instance, weight)` (the function `pose.rs` hooked as "ground IK"), and `0x140393F60` ends it (Checked (decomp)). Both starts are hooked (prologue `48 89 5C 24 18 55 56 57 41 54 41 55 41 56 41 57`, Checked (exe)): after KH1's call on Sora the same KH1 function runs on the holder instance with the holder motion (t702), the restart only while the holder itself blends; the existing blend-pass hook runs KH1's pass on the holder at Sora's weight, and KH1's blend end runs on it before a plain pass. So the left Keyblade's hold on the hand blends over the same frames as his hand. The holder's local matrices are a block of KH1's heap (`0x14028DA20`), checked against KH1's block list (`0x142B9C280`) before each pass because mode starts set the heap up anew (`0x14028D9B0`). Ini `left_blade_blend` (on; off = t702's instant switch, for comparing). Assumption: the holder motions carry no IK chains. Not checked in game. docs/kh1-weapon-second.md §10.4.1.

### 2026-10-26: Stage 712 (testing branch: In-game Scene Skip in QoL)
- **In-game Scene Skip (QoL row: Off | On, ini `ingame_skip`, off by default).** The user: "you can make it a seperate config option in qol, what I want you to do is have the cutscene skip ... it skips that in game cutscene fully, then if any flags give you an item the game gives you the item after the fact". The low-poly dialog scenes go through KH1's own skip, the one the pause menu's Skip Cutscene runs: bit 0 of the second scene flag word (`0x142382594`) and the flag byte (`0x14232DF89`) make KH1's script step (`0x1401A2320`) run its skip (`0x1401A22B0`); in skip mode the frame step is 128 and the message functions open nothing, while the item and ability give (event function 583) and the story flags run as usual, and KH1 ends skip mode itself when the entry ends (`0x1401A2130`) (Checked (decomp + exe)). `ingameskip.rs`: the first time an event entry (not 0, `0x142382524`) is in event mode (`0x400`), it reads every thread's code once (contexts `0x1423902D0`, count `0x14237F4E8`); the pause skip's functions (84, 139, 476, 489, 623; not 27 and 317, which set the bit only under a condition and 27 runs in the low-poly scene) leave the scene to Cutscene Skip, a message choice (88) or a shop / menu window (222, 268, 278, 392, 628, 629, 673) or a talk's bit (`0x800`) make it play, anything else is skipped. The bit 0 it set is cleared when the entry is back to 0 or skip mode ends; `cutsceneskip.rs` ignores it (one line). No hook; read in the pad read (`probe::pad`), one atomic load when off. Steam only; KH1's skip, the step's bit test and flag byte read / clear, and the table entries of every event function named are checked before use. The t709 / t710 logs: the Traverse Town low-poly scene (room 3 0 4, entry 1) had `0x400`, no bit 0, no `0x800`, none of the pause skip's functions or a choice, and its item message (605) came before the give (583); 139 ran in all ten high-poly scenes (Checked (trace)). Not run in game; Assumptions: every high-poly scene has a pause-skip function in its own entry, a chained scene is skipped too, the item popup is not shown (the item is given). Docs: `docs/kh1-ingame-skip.md`, `docs/kh1-config-menu.md` (6.z5).

### 2026-10-26: Stage 711 (testing branch: Cutscene Skip fires at once)
- **Cutscene Skip, no wait.** The user tested t710: "it worked for the high poly cutscenes but it waits awhile, it should skip the first moment the cutscene loads". The one second wait is removed: the moment KH1's cutscene bit (bit 0 of the second scene flag word) comes on, the mod writes the skip flag byte and KH1's own script step runs its skip. The user's t710 log showed five skips, each written 30 log frames after the bit came on (the wait) and each followed by runner 2 to 3 and the scene ending, as after the pause menu (Checked (trace)). The scene's script starts some frames before the bit comes on (15 to 100 in that log), and the bit is the earliest sign KH1's script step will accept a skip, so that is the earliest this can fire (Checked (decomp + trace)). Not run in game. Docs: `docs/kh1-cutscene-skip.md`. The user's second report ("the in game cutscene skip didn't work", the low-poly dialog scenes) is a separate question, not part of this version.

### 2026-10-26: Stage 710 (testing branch: Cutscene Skip in QoL)
- **Cutscene Skip (QoL row: Off | Seen | All, ini `cutscene_skip`, off by default).** The user: "an auto cutscene skipper in qol", never item shop text, NPC talks or anything the player starts; skip the dialog cutscenes with high-poly models that the pause menu's Skip Cutscene can skip, using the game's own function; never the low-poly ones confirmed line by line (they can give items); answers: Seen = already-watched scenes (look for the game's own seen flags first), a one second wait, hook-or-read as found. Found in the decomp and the user's two logs (Checked (decomp + exe + trace)): a skippable cutscene is bit 0 of KH1's second scene flag word (`0x142382594`); the pause menu's Skip Cutscene only sets one flag byte (`0x14232DF89`) and KH1's own script step then runs its own skip (`0x1401A22B0`), as the trace shows after the menu. `cutsceneskip.rs`: with the row on, one second after the bit comes on it writes that byte (if the bit is still on); KH1 does the rest. No hook: the bit is read in the mod's pad read (`probe::pad`), one atomic load when off. The setter (`0x1401A0E60`) was not hooked (its first instruction is position dependent, there are 13 callers and several direct clears), so this is the pad-read poll (the user's third answer was a hook if found). Steam only; the skip function's bytes and the step's read and clear of the byte are checked before use. Seen: KH1 keeps no seen flags (Checked (decomp): strings and the skip path searched), so the mod keeps its own list in the ini (`cutscene_seen`: room, event file and the scene's order in the room visit; shared by all saves; Assumption). Not run in game. Docs: `docs/kh1-cutscene-skip.md`, `docs/kh1-config-menu.md` (6.z4).

### 2026-10-26: Stage 709 (testing branch: Cutscene log watches flags and buttons)
- **Cutscene log, flag and button watch (no behaviour change).** The user explained that high-poly dialog cutscenes can be skipped from the pause menu ("Skip Cutscene") and the skipper should call the game's own function, while low-poly dialog scenes (confirm through every line, sometimes give items such as dodge roll) and talks must never be skipped. The skip function is not found yet (no text for it in the decomp). With the Cutscenes log on, `cutscenelog::frame` (called from the debug frame end, debug tools only) logs a `watch:` line whenever one of KH1's scene flag words, the engine flag word, the event-file player and the NPC context count, or the pad buttons change, so pausing, choosing Skip Cutscene and pressing confirm show which words move. Off costs one atomic load. Word meanings are Checked (decomp); which bits mean what is an Assumption. Not run in game. Docs: `docs/kh1-cutscene-log.md`. The skipper waits for this log.

### 2026-10-26: Stage 708 (testing branch: Cutscene log reads the right object)
- **Cutscene log, second pass (no behaviour change).** The user sent a log of Traverse Town's events before and after Guard Armor and asked that the skipper never skip item shop text, NPC talks or anything the player starts. t706's script lines were unreadable: `0x1401A38A0` takes the script file and an entry (not a context), so its context was read wrong; scene prepare and movie never fired (events here are room scripts, not scenes) (Checked (trace + decomp)). `cutscenelog` now logs each script start with the file, the entry, KH1's scene flag words and what the script holds, read from KH1's fixed contexts (`0x142382AD0` entry 0, `0x1423902D0` others): sub-scripts, length and distinct instructions (class / operand). Still the Cutscenes category on the debug menu's Logs page, off by default, debug tools and Steam only. Not run in game. Docs: `docs/kh1-cutscene-log.md`. The skipper itself waits for a log with the user's labels.

### 2026-10-26: Stage 707 (testing branch: Soft Reset in QoL)
- **Soft Reset (QoL row).** The user: "in qol you can quick reset the game with l1 l2 select and start pushed together", then "the combo does not work in the pc version so restore it" (the PS2's L1 + L2 + R1 + R2 + Start + Select). New QoL row Soft Reset (Off | On, ini `soft_reset`, off by default; `win/softreset.rs`): the combo, read from the mod's pad read (`probe::pad`), held a second sets KH1's game mode (`0x14233FBC0`) to 1 with its timer (`0x1422EC0A8`) below zero and no pending mode change, so KH1's own mode step runs its own full reset (`0x140170630`) and loads the title screen. No hook; one flag check in the pad read when off. Steam only; the first bytes of the mode step and the reset and the mode's address are checked once. Checked (decomp) for the flow and KH1's own combo mask `0x0F09` (which does nothing in this build, per the user), (exe) for the bytes; not run in game. Assumption: KH1's boot reset clears whatever a running world leaves. Docs: `docs/kh1-softreset.md`, `docs/kh1-config-menu.md` 6.z3.

### 2026-10-26: Stage 706 (testing branch: Cutscene log, debug tools only)
- **Cutscene log (no behaviour change).** The user: "add an auto cutscene skipper in qol" (events and movies, a setting for each), then "go ahead, make sure the log is toggled in the debug menu so we don't bloat performance". New log category Cutscenes & Events (`LogCat::Cutscenes`, ini `log_cutscenes`, off by default, the debug menu's Logs page) and `win/cutscenelog.rs`: with the debug tools on and the Steam build, three hooks that only log (each checks its function's first bytes and refuses on a mismatch): KH1's scene prepare (`0x17CB10`: arguments, event flag, scene triple), event script runner (`0x1A38A0`: code, counter, opcode, end) and movie opener (`0x11C640`: file name; Assumption: the second argument is the path). Checked (decomp) for the layouts and (exe) for the bytes; not run in game. 40,000-line cap. Docs: `docs/kh1-cutscene-log.md`, `docs/kh1-debug-menu.md`. The skipper itself comes in later versions, after a log.

### 2026-10-26: Stage 705 (testing branch: Config menu: Command Menu list with L2 Stick Menu)
- **Config menu: Command Menu list with L2 Stick Menu.** The user: "another config menu called command menu and put l2 stick menu in there". KH1 in KH2 is now Camera, Combat, Movement, Command Menu, QoL, Cosmetics, Misc; the new list's one row is L2 Stick Menu, moved from Camera (same setting, ini key and help). Config rows only, no game code touched. Assumption: seven top rows fit above the help box at KH1's size. Not checked in game. docs/kh1-config-menu.md §6.z2.

### 2026-10-26: Stage 704 (testing branch: Config menu: QoL and Cosmetics lists, hit-reaction and damage rows into Tweaks)
- **Config menu: QoL and Cosmetics lists, hit-reaction and damage rows into Tweaks.** The user: "a qol (Quality of life) section in the config menu, put gummi skip in there, then put recoil, critical, party critical, kh2 damage, medium hits, knock back numbers, and hit reactions into the tweaks section", and "in the main sub menu make a new menu called cosmetics and put a setting in there for keyblades to always have their trail on". KH1 in KH2 is now Camera, Combat, Movement, QoL, Cosmetics, Misc. Combat: Physics, Base Combo, Aerial Base Combo, Magic, Tweaks. Tweaks (eleven rows): KH1 Ability Chance, Enemy Weight, Cast from Glide, Clash Combo, then Hit Reactions, Knock-back Numbers, Medium Hits, KH2 Damage, Criticals, Party Criticals, Recoil (moved from Combat; each keeps its setting, ini key and help). QoL: Gummi Routes (from Misc). Cosmetics: Keyblade Trail (Normal | Always, ini `trail_always`, default off; `cosmetics.rs`): ahead of KH1's own trail step (`0x1402C1F20`) for Sora, while his weapon is drawn, the trail bit (`+0x1CC` bit 23) is set and, with no trail running or asked for, KH1's trail request (the weapon object's `+0x30` / `+0x38`, as `0x1402C2310` writes it) is written with row 13's trail id, so KH1's step makes, keeps and ends the trail; Valor's and Final Form's left Keyblade the same through `leftblade`'s trail step (Checked (decomp)). New `List::QoL` / `List::Cosmetics`, text ids `0x7F000211..18`. Assumptions: row 13's trail id suits every Keyblade and form; the eleven-row Tweaks list at its smaller scale reads well. Not checked in game. docs/kh1-config-menu.md §6.z, docs/kh1-cosmetics.md.

### 2026-10-26: Stage 703 (testing branch: Base Sora's growth and abilities from Drive Form levels)
- **Base Sora's growth and abilities from Drive Form levels.** The tester (what-is-still-missing, after m15, answer 6): "(c)". On a form's growth rise, KH2's level-up (Steam KH2 `0x1403E8380`) swaps Sora's own growth entry from two under the form's new level to one under it, an empty slot becoming LV1 (table `0x1405C9030`), and gives the row's fmlv ability through `0x1403C3F00` (Checked (decomp + exe)). With the player's fmlv: High Jump / Quick Run / Glide LV1 / 2 / 3 at Valor / Wisdom / Final LV3 / 5 / 7, never MAX, and abilities at LV2 / 4 / 6 (Checked (files)). Base Sora's level for each is now the higher of the mod's own rule (his level, bosses) and the form's level less one (`formlevel::sora_growth`, `formexp::base_growth`, `growth::FormGrowth`); without the ability, KH1's High Jump goes into the shared list (on), Quick Run into Sora's list (off, its AP in the bonus AP), KH1's Glide for Final. Rewards come as KH1's own counterparts, off, each with a "New Ability" box (`growth::kh1_reward`): Combo Plus 162 → 6, Air Combo Plus 163 → 7, MP Rage 412 → KH1's MP Rage 24; the Auto forms, MP Haste and Form Boost aren't given yet (asked). The form's save record keeps the last level whose ability was given (bits 10..12; EXP now 10 bits, KH2's largest step 672); t700 saves read as nothing given, so their rewards come once. `abilities = kh1` only, in KH1's menu step. The form level cap was re-checked against `0x1403C3EF0` / `0x1403C4720` (item ids at `0x1405C5D08`) and is already KH2's rule (answer 5): no change. Assumptions: KH1's MP Rage stands for KH2's; the growth entry's equip bit doesn't matter here. Not checked in game. docs/kh1-drive-forms-native.md §9, docs/abilities.md.

### 2026-10-26: Stage 702 (testing branch: Left Keyblade: its bones posed by KH1's own pose pass)
- **Left Keyblade: its bones posed by KH1's own pose pass.** The tester (what-is-still-missing, after m15, answer 3): "(a)" (bake the left hand's weapon bones into Valor's moves so KH1's own animation drives them). KH1 Sora's skeleton has one weapon-bone set (280..292), and a motion has one channel record per model bone (pass `0x140391C70`, reset `0x14038F550`, bone pass `0x1403900B0`; Checked (decomp)). So each Valor move gets a second KH1 motion of the same length, its holder motion (`kh2slots::holder_one`, its own slot-cache key; the main bakes are unchanged): its body at KH1 bind, its weapon bones KH2's left Keyblade (`W_EX010_BTLF_L`, through `leftblade::left_bones`, the rule `note_pose` ran each frame), held from the left hand. Right after each of Sora's passes (plain: its motion and time; blend: actor `ptr(+0x1D4)[+0x168]` at `+0x16C`), KH1's own plain pass poses it on an instance of ours (own `+0x1B0` / `+0x1B4` / `+0x1B8` arrays, Sora's model slot at `+0`, `+0x14` = 0: no ground lift, no look-at). The left Keyblade's bones are Sora's drawn left hand × the holder's hand⁻¹ × the holder's bones, so it stays in his hand through KH1's blends; KH2's left weapon is no longer evaluated each frame on those slots. Final Form, and any move whose holder couldn't be baked, keep `note_pose`. Assumptions: KH2's left grip isn't mirrored (a mirrored move is logged and kept on the old path); nothing reads the pass's scratch globals between Sora's pass and the next actor's. The holder actor copy for KH1's draw and trail is still ours. One bake of the holder motions on first start. Not checked in game. docs/kh1-weapon-second.md §10.4.

### 2026-10-26: Stage 701 (testing branch: Clash Combo (Tweaks): KH2's combo count after clashes and wall bounces)
- **Clash Combo (Tweaks).** The tester (what-is-still-missing, after m15, answer 1): "make it a toggle in the config menu under tweaks, you can either have the kh1 rule or the kh2 rule". KH2 (Steam KH2, Checked (decomp)): three functions set "connected" (`+0x120` bit 0x2000000) and take one off the combo count `+0xD74` (if above 0): `0x1403D61D0`, the clash reaction ("connected" always; one off only on its recoil branch 41 / 43, not the guard repel 42 or refact 2), and `0x1403D54B0` / `0x1403D63E0`, the background-hit handler (vtable `+0xD8`) of two player-class families, called by the wall contact `0x1403D1960`: the "two other cases" are the wall bounce. KH2's 41 has its combo window from frame 16, 43 none (Checked (files)). KH1 (Steam KH1, Checked (decomp)) already sets Sora's "connected" word (combat info `+0x50`, via the attack's `+0x60`) when its clash half `0x1402B4BC0` / meeting `0x1402B4A20` stops his attack, and at its wall contact `0x1402B3A00`; its ground recoils `0x14029D720` / `0x14029D5F0` take one off `+0x41` (no floor), the air one `0x14029D510` none; its slot 72 has a combo window from frame 28 (Checked (files)). New Combat > Tweaks row Clash Combo (KH1 | KH2, ini `clash_count`, default KH2 as the other Tweaks rows). At KH2: after KH1's meeting (the existing `recoil::meet` hook) Sora's swing counts as connected once both of KH1's directional tests `0x1402B4640` pass, also when KH1 doesn't stop it (priority 0x62+); after KH1's hit pass (`recoil::after_pass`) a recoil (0x6E / 0x6F / 0x70) leaves the count one below its value before the pass, floored at 0. At KH1 nothing changes. No new hook. Assumption: KH1's pairing, overlap and directional tests stand in for KH2's table and angle checks. Not checked in game. docs/kh1-kh2-recoil.md §6.10.

### 2026-10-26: Stage 700 (testing branch: Drive Form levels in KH1)
- **Drive Form levels in KH1.** The tester (what-is-still-missing, after m14, answer 6 (b)): "build form levels into KH1". KH2's form EXP and level rule (Steam KH2 `0x1403E8380`: levels taken while under the cap, each fmlv row's EXP off, EXP 0 at the cap; cap 2 + forms owned `0x1403C3EF0` / `0x1403C4720`; growth table `0x1405C9030`; Checked (decomp/exe)) runs in `formlevel` / `win/formexp.rs`, fmlv rows read at run time from the player's own 00battle. Records in Sora's save at `+0x882` / `+0x884` / `+0x886` (Valor / Wisdom / Final, u16: level top 3 bits, EXP low 13; 0 = LV1): these follow the bool `+0x880` at the end of KH1's first save block (field list `0x1402CF620` / `0x1402CF7E0`), and no instruction accesses them (Checked (exe); Assumption: no computed-pointer access). Valor: +1 per record Sora's attack makes on an enemy at KH1's contact `0x1402BF330` (KH2 `0x1403E86B0`). Wisdom: +1 per enemy Sora last hit that KH1's enemy-script EXP command defeats (`0x1402BC9A0` / `0x1402BCA20` hooked at entry, prologues checked; KH2 `0x1403E8780`; Assumptions: one call per defeat, KH1 enemies count as Heartless). Final: nothing (KH2 counts Nobodies; KH1 has none). The form's growth ability is fmlv's growth for its form level (LV1/2/3/MAX at form LV1/3/5/7), replacing MAX and Sora+1. A "Form Level Up" prompt box. Sora's own growth from form levels and the rows' rewards are not given (asked). Not checked in game. docs/kh1-drive-forms-native.md §9, docs/drive-model.md.

### 2026-10-26: Stage 699 (testing branch: Left Keyblade: its trail by KH1's own trail request and step)
- **Left Keyblade: its trail by KH1's own trail request and step.** The tester (what-is-still-missing, after m14, answer 2). Before, our code made the trail (`0x1402AE290`), set its end byte and kept up to four. Now KH1's trail request `0x1402C2310` (already hooked) and trail step `0x1402C1F20` (new hook, prologue `40 57 48 83 EC 20`) each run for the holder right after Sora's: every request KH1 gets for Sora is asked for the left object too (the left Keyblade's own effect data and look), and KH1's step makes, fades and ends the trail by the holder's `+0x1CC` bit `0x800000`; out of the form it keeps running with the bit off while a left trail lives, so KH1 fades it. The left trail window on KH1's states is a new level motion event, 7.1, baked into Valor's motions from the left weapon's trail windows (`WindowKind::WeaponTrail { weapon }`; the right one's 7.0 unchanged), read with KH1's level reader `0x140394770 (motion, 0x70001, time)` on Sora's motion at `+0x16C`. Vanilla KH1 never asks for 7.1 (its pass reads only an exact 7.0; its edge reader `0x140394900` types 2/3/4/6). Final Form (KH2's controller) keeps the KH2 data's windows. Checked (decomp, exe). Assumptions: `+0x16C` is the time KH1's pass reads levels at; the holder's copied flags stand for Sora's. One rebake of Valor's moves. Not checked in game. docs/kh1-weapon-second.md §10.3, docs/kh1-anim-slots.md.

### 2026-10-26: Stage 698 (testing branch: Left Keyblade: drawn by KH1's own actor draw)
- **Left Keyblade: drawn by KH1's own actor draw.** The tester (what-is-still-missing, after m14, answer 2). Before, our model-queue hook called KH1's queue a second time with the left matrices written over Sora's bones. Now KH1's party actor draw `0x1402A1630` is hooked (prologue `40 55 53 56 57`): after Sora's own draw it runs again with KH1's own arguments on the holder, mask `0xFFFFFFFF` (every body mesh hidden); KH1's own test (flag `0x4000`, object and model) decides whether the weapon is drawn and KH1 queues the object's model and colour on the holder's weapon bones, which hold the left hand's matrices for that call only. `0x1402A1630` draws no shadow and writes nothing to the actor but the queue's `+0x374` bits 30/31 on the non-HD path, which land on our copy. The pause-menu copy is still skipped. Checked (decomp, exe). Still ours: the holder (a copy of Sora's actor and instance carrying the left hand's bones, plan step 4). Not checked in game. docs/kh1-weapon-second.md §10.2.

### 2026-10-26: Stage 697 (testing branch: Left Keyblade: its weapon object from KH1's own pool)
- **Left Keyblade: its weapon object from KH1's own pool.** The tester (what-is-still-missing, after m14, answer 2): "all three, make it native to kh1 like the base keyblade". The left Keyblade's weapon object was a 0x60-byte copy of Sora's in our own block; it is now one of KH1's own nine (pool `0x14296B1E0`, in-use bits `0x14296B5D8`), filled as KH1's `0x140286D60` fills a party member's (`+0x18` effect data, `+0x20` weapon row, `+0x40` / `+0x48` cleared, `+0` the container's handle, `+4` via `0x1401D6760`, prologue checked), `+0x38` −1. Checked each use to still be ours, made again after KH1's area start (`0x140286B40`, which clears the bits), colour `+0x0C` from the right object's, `+0x18` from the right Keyblade's when the left file has none; given back by clearing its bit, as `0x1402868E0`, when the left model is released. The holder's `+0x14C` is this object, so `lefthits`, `kh2dpx` and KH1's bone point `0x14029EB20` read KH1's own object. Steam only (on Epic not installed, logged). Checked (decomp, exe). Assumption: −1 at `+0x38` is "no request". Not checked in game. docs/kh1-weapon-second.md §10.1.

### 2026-10-26: Stage 696 (testing branch: KH2's ground moves carried off a ledge)
- **KH2's ground moves carried off a ledge.** The tester (what-is-still-missing, m14 answer 4): "(a) and can you make it in the debug". KH2's mover (Steam KH2 `0x1403B9090`) cuts a ground move into the fall only when no floor is under him and no no-gravity range (ids 3 / 24 / 30) or edge stop (29) is open (`0x1403BA9A0`, `+0x18C` bits 5 / 6 / 23 / 24); while one is open the move goes on (Checked (decomp)). KH1's physics step (`0x14029A3C0`) cuts on state 0's transition (`0x1402CC470`, table `0x142DB3840 + 0x28`) returning 2: action flag off, fall from rest (`0x1402AF8F0`), speed capped at 8, which is KH2's cut (Checked (decomp)). That transition is now wrapped in KH1's table (`shared::physics::ground_next`, Steam only, slot and prologue `40 53 48 83 EC 20 48 8B D9` checked, Checked (exe)): for Sora's ground action on our slots with such a range open (`kh2slots::ground_hold`) it stays 0, so his ground step and root travel carry him off, and KH1's own cut follows the first frame they shut. Moves (Checked (files)): Valor 154 (all of it) and 155; base Sora 154 / 155 / 161 / 165 / 169 (edge stop) and the Drive 52. KH1's own moves unchanged. New debug-menu row Ledge Carry (ini `debug_ledge_carry`, default on). Assumptions: KH2's entry / `+0x120` bits are clear for Sora; KH1's ground pre-step and mover keep him as KH2's with no floor. Not checked in game. docs/kh1-drive-forms-native.md §7.4.5, docs/kh1-debug-menu.md.

### 2026-10-26: Stage 695 (testing branch: KH2's pad vibration on KH1's vibration player)
- **KH2's pad vibration on KH1's vibration player.** The tester (what-is-still-missing, after m14, answer 9): "(a) if rumble is on it should work then". KH2's frame trigger 31 is the pad vibration: `0x14040DD70` case 0x1F → `0x1403EEFC0` (bsearch in `00battle.bin` vbrt's `vibr`, the index-th 0x2F pattern) → `0x14019AE60` / task `0x14019B1D0` (motor bytes `+0x48` / `+0x49`, level(next key) × frac at 30 Hz ticks) (Checked (decomp + exe + files)). Every baked slot's trigger 31 is a sound-event key of ours (`ks::KH2_RUMBLE` 0x84, the pattern id), fired by KH1's motion pass; `kh2slots::sound_event` → `rumble::play` writes KH2's pattern once in KH1's `btlvibe` layout for the frame scale (`kh1_data::vibe`) and starts it with KH1's vibration start `0x1401E1400 (pattern, 0)` (prologue checked). KH1's task 20001 steps it; KH1's pad read sends it only with Config's Vibration on (`0x14028B0B0`, `0x1402FB030`) (Checked (decomp)). Moves: reactions, recoils, guard repel, the Drive, casts, Explosion, Finishing Leap, Counterguard, Magnet Burst, Wisdom's 151 / 154 / 181 (Checked (files)). Levels match KH2's from the second step at 60 and 30 fps (`vibe::tests`). Assumptions: KH2's motor channels 0/1 are KH1's 0/1; KH2's motors read 0 after a pattern. Steam only. One rebake. Not checked in game. docs/kh1-anim-baking.md §13.6.

### 2026-10-26: Stage 694 (testing branch: Drive Forms: the form's stance after its arrival)
- **Drive Forms: the form's stance after its arrival.** The tester (what-is-still-missing, after m14, answer 3 (a)): add KH2's stance (motion 6) after the arrival on the ground. KH2's new form's first think (Steam KH2 `0x140405630`) plays the arrival 53, queues motion 6 with an 8.0 blend, then the idle with none, for every form, only on the ground (Checked (decomp)). Valor's B010 (15 frames at 30 fps) and Wisdom's C010 (20) are free all through, range id 2 (Checked (files)). A form-only row, "Form stance" (`kh1rows::STANCE_MOTION`), is baked for every form on KH1's states with a motion 6; `form::native_arrive` (`ask_own_then`) queues it after the arrival with bit 30 and blend 8.0, then KH1's idle with blend 0. KH1's ground step `0x1402A9740` asks for the idle only with no speed and an empty queue, and the stick empties the queue (`0x14029F7C0`) (Checked (decomp)), so standing plays the stance through and the stick leaves it, as in KH2. In the air unchanged. Assumption: KH1's blend is in 60 Hz frames. One rebake. Not checked in game. docs/kh1-drive-forms-native.md §7.4.3.

### 2026-10-26: Stage 693 (testing branch: KH1's picker refuses as KH2's walk does)
- **KH1's picker refuses as KH2's walk does (step 1).** The tester (what-is-still-missing, m14 answer 8): "patch the features kh1's combo row and other related logic are missing ... so that valor and wisdom can behave correctly without overwriting kh1's vanilla logic". Steam KH1's Attack step `0x1402A7510` hands the picker's return (`0x1402A6EE0` / `0x1402A6C70`) straight to the row runner `0x1402A68F0` in states 0 / 2, the return + 3 in flight (Checked (decomp)). When KH2's walk takes no row, KH1's picker now answers "no attack" itself (`kh1picker::at_kh2`, `combomods::Ask`, `NO_ROW`) and the row-runner wrapper (`combat::exec`) runs nothing: one rule for base Sora at Base / Aerial Base Combo KH2 (before: KH1's own row ran) and the Drive Forms (before: t685's check at the row runner). Not in flight, not when the press has a Square move or Counterguard to run, not if the wrapper isn't installed. A refused press clears KH1's "connected" word `+0x50`, as `0x1403D6CA0` clears its bit (Checked (decomp)). A press KH2 couldn't be asked about keeps KH1's own row for base Sora. The row runner no longer asks KH2's walk a second time when the picker is hooked. Assumption: a refused walk leaves KH1's count where its step put it. Still running along: the walk on our controller object and the Finishing Plus counter (plan in the doc). Combo rows at KH1 unchanged. Not checked in game. docs/kh1-drive-forms-native.md §7.4.2.

### 2026-10-26: Stage 692 (testing branch: KH2's rule for an Attack after a missed swing)
- **KH2's rule for an Attack after a missed swing.** The tester (what-is-still-missing, after m14): "7. (a)". Steam KH2's input step `0x1403D6FC0` runs the Attack command through `0x1403D6CA0` with "connected" = `+0x120` bit 0x2000000 or Combo Master, with no airborne case: in the finisher window (range id 50), connected, a finisher left: the next finisher; in the combo window (range id 11), connected: the next swing; else a free range (ids 0/1/2) starts a new combo; else nothing, no buffer (Checked (decomp)). Combo Master is `[+0x5C0]+0x1E4` bit 13 (ability number 173), Berserk Charge `+0x1DC` bit 11 (107), read through the bit set `+0x1D0` (`0x140402750`) against `03system.bin` (Checked (decomp + files); both were Assumptions). No KH2 swing or finisher has a free range (Checked (files)). KH1's Attack step `0x1402A7510` already follows this on KH1's states (combo window `+0x1CC` bit 22, "connected" `+0x50`, KH1's Combo Master 0x41; Checked (decomp)), so base Sora, Valor and Wisdom were right. Our KH2 controller's walk (`Character::plan_press`, still used by Master, Final, Limit and Anti) took whiffed air swings as connected and restarted a whiffed ground swing in its window; it now follows `0x1403D6CA0`, and `Abilities::one_swing_on_miss` is gone. Tests `whiffed_air_swings_do_not_chain`, `whiffed_swings_do_not_advance_the_combo`. Not checked in game. docs/kh1-drive-forms-native.md §7.1.2, docs/movement-model.md.

### 2026-10-26: Stage 691 (testing branch: Wisdom's magic on KH1's cast state)
- **Wisdom's magic on KH1's cast state.** The tester (same). In Wisdom on KH1's states Fire, Blizzard, Thunder and Cure are KH2's (`formmagic::wisdom_kh2_spell`, Config rows ignored), cast by KH1's command step on Wisdom's own cast slots (`WISDOM_SPECS` with `CASTS`, its finisher casts 57 / 60 / 63 built per form, `pick_cast` / `cast_motion` by the moveset playing), KH2's script started by KH1's spell start hook (`0x14028F140`) at Wisdom's sub (+10, Checked (files): rows at sub 10). KH1's Gravity, Stop and Aero play on Wisdom's cast motions with KH1's scripts. Not checked in game. docs/kh1-drive-forms-native.md §8.3.

### 2026-10-26: Stage 690 (testing branch: Wisdom's shots from its slots on KH1's states)
- **Wisdom's shots from its slots on KH1's states.** The tester (same): "finish valor and wisdom form". In Wisdom on KH1's states a frame trigger 1 of its slots, fired by KH1's motion pass (`0x140295420`, wrapped since t677), runs the form's `wisd` script (`effects_at` → `form::effect` → `formmagic::effect`, t687's per-frame effect keys) instead of Sora's effect package; `takeover::native_spells` runs the script's frame in a native form with its clips. The shots hit as KH1 attack objects in KH1's own pass (`bodyhits::spell_attacks`), setting Sora's "connected" word. Still running along: the shot's flight is our interpreter's (`kh2-magic`); plan in the doc. Checked (files) the triggers. Not checked in game. docs/kh1-drive-forms-native.md §8.2.

### 2026-10-26: Stage 689 (testing branch: Wisdom Form on KH1's own states)
- **Wisdom Form on KH1's own states.** The tester (what-is-still-missing, after m13): "finish valor and wisdom form ... make sure when its in the new system that its 1:1 with kh2, our configs don't adjust anything on drive forms". Wisdom (`P_EX100_MAGF`, moveset 3) runs on the per-form path: `kh2slots::WISDOM_SPECS` (151 on 0xC8..0xCA, 154 on 0xCB, 181 on 0xCC..0xCE, reactions, item, fall, landing; jump 3 on both takeoffs, KH2's jump start plays 201 only with High Jump, Steam KH2 `0x140422630`, Checked (decomp)), its combo rows, arrival and Quick Run 203 / 204 on our rows; its `prty` (walk 2, run 12, jump 185, acceleration 0.85, brake 0.97; Checked (files)). New ground inertia on KH1's ground step (`shared::physics::ground_inertia`): KH2's free-state move `0x140422350` → `0x1403B7AD0`, target×(1−a)+v×a with the stick, ×brake without (`0x140141AE0`, Checked (decomp)), per KH2 tick; inside an action steered in the clip's allow-movement range, braked outside (Assumption). Square is Quick Run (KH2's handler `0x1403A81F0`: no glide, Square rows, row 11 or roll in Wisdom, Checked (decomp)), one level above Sora's. One rebake (Wisdom's files in the manifest). Not checked in game. docs/kh1-drive-forms-native.md §8.1.

### 2026-10-26: Stage 688 (testing branch: Drive Forms on KH1's states: one path for every form)
- **Drive Forms on KH1's states: one path for every form.** The tester (what-is-still-missing, after m13): "if something we've implemented is the same way for drive forms make it a part of that code". t678's Valor-only pieces are now a per-form table: `form::Kind::native_capable` / `native_form`, `kh2slots::FORM_SLOTS` (each form's specs and extra rows of ours, built by one loop in `prepare`), `Move::form` as the form (`clip_of`, `live`, `form_of`, `form_built`, `redirect` and the sets' `form_requests` read it), `pose::Kh2::form_src`. The slot cache's key is the form's model name, so Valor's keys are as before. Valor's behaviour unchanged. Checked (code). docs/kh1-drive-forms-native.md §7.5.
  Also: t687's effect keys follow the form he is in (`native_form`), not Valor only.

### 2026-10-26: Stage 687 (testing branch: Valor's effects on KH1's own states)
- **Valor's effects on KH1's own states.** The tester (what-is-still-missing, after m13): "currently I see: no effects". The wrapped motion-event handler (Steam `0x140295420`, t677) started every KH2 effect key from Sora's package and only for the `kh2_effects` moves, so none of Valor's played (its element ids are its own archive's, `P_EX100_BTLF.a.us`); and KH1's 8 edges a call (`0x140394900`) dropped part of the Drive motion 52's 13 frame-0 elements and Valor's arrival 53's 15 (Checked (files)). KH2 fires every trigger of a frame in one update (Steam KH2 `0x14040E960`) and starts trigger 1's element from the motion owner's own package (`0x14040DD70` case 1, `+0x148`) (Checked (decomp)); Valor's weapon motions have no trigger 1 (Checked (files)). Now the effects are one type-6 key per frame (`KH2_EFFECTS` 0x83, `c` the frame and whether the slot is Valor's); `kh2slots::effects_at` starts that frame's elements from the moveset's own clip, Valor's through `form::effect` (Valor's package and Keyblade anchors), Sora's as before. Sounds unchanged. Rebakes the affected slots once. Assumption: trigger 31 on 52 / 41 is the pad rumble (not added). Not checked in game. docs/kh1-anim-baking.md §13.5, docs/kh1-drive-forms-native.md §7.

### 2026-10-26: Stage 686 (testing branch: Valor's arrival held in the air)
- **Valor's arrival held in the air.** KH2's form think `0x140405630` plays the arrival 53 in the
  air in ACTION_FLY with velocity 0 (no vertical step, `0x140421C80`). It then falls from rest
  (motion 4, `0x1403CBC30`'s hop of 0). Checked (decomp).
  - Our 53 has no no-gravity range, so he fell through it.
  - Now `form::arrival_held` holds him on KH1's air state with no speed, and the fall restarts from
    rest after.
  - Assumption: `+0xE10` 2 means "changed in the air".
  - Open: on the ground KH2 plays Valor's motion 6 (B010) before the idle; it isn't baked.
  - docs/kh1-drive-forms-native.md §7.4.3, §7.4.4.

### 2026-10-26: Stage 685 (testing branch: No KH1 attack row in a Drive Form when KH2 takes nothing)
- **No KH1 attack row in a Drive Form when KH2 takes nothing.** KH2 refuses a press its rule or
  rows don't take (`0x1403D6CA0`, Checked (decomp)).
  - Before, KH1's own picker row ran instead: in the air, KH1's own hop of 40 / 20 and lunge of
    8 / 12 / 4, with Valor's motion on it.
  - Now it is no attack (logged).
  - Assumption: KH1's count stands for KH2's free-state reset. Base Sora's same fallback at KH2 is
    left as it is (asked).
  - docs/kh1-drive-forms-native.md §7.4.2.

### 2026-10-26: Stage 684 (testing branch: Drive Forms ignore the combo styles on KH1's states)
- **Drive Forms ignore the combo styles on KH1's states.** The tester (what-is-still-missing,
  after m13): "attack doesn't interrupt jumps momentum so you can fly high in the air while
  attacking", and "our configs don't adjust anything on drive forms".
  - KH2 replaces the jump's arc and velocity with each air row's hop (Steam KH2 `0x1403D78D0` ->
    `0x1403CBB00`, Checked (decomp)). Every one of Valor's air rows hops (PTYA moveset 2: 182/181
    35/40, 184/183 14; Checked (files)).
  - `combomods` read the Base / Aerial Base Combo rows itself. At KH1 it dropped the KH2 pick that
    KH1's picker had made for Valor, and ran the row with no hop: the High Jump's rise went on
    under the swings.
  - Now `combomods::style_kh2` is KH2's in a Drive Form on KH1's states (length, pick, row
    selection, line ahead, ground hop). Base Sora is unchanged.
  - Not checked in game. docs/kh1-drive-forms-native.md §7.4.1.

### 2026-10-26: Stage 683 (testing branch: Valor's left Keyblade hits with KH1's own Keyblade sphere)
- **Valor's left Keyblade hits with KH1's own Keyblade sphere.** The tester (what-is-still-missing, after m13): "1. it should fit kingdom hearts 1 in this situation for both keyblades", "2. the keyblade should behave exactly like kh1's keyblade so make sure it scales by keyblade", "for 2nd keyblade it should behave 1:1 with how the main keyblade works". KH1 builds its Keyblade object as follows (Checked (decomp)):
  - the constructor `0x1402B2DC0` takes bone `0x30`, blade range `+0x54` / `+0x58` and radius `+0x5C` from the data (floats, Checked (exe));
  - the row maker `0x1402B3270` adds the weapon row's radius, length factor, priority and critical;
  - `0x1402B3800` takes the blade's ends with `0x14029EB20` on the object's bone-source actor `+0x90`, through that actor's weapon row's blade range, and `0x1402B3D50` lays its spheres between them.

  The right Keyblade already is that object. The left object is now made the same way:
  - its data keeps the swing row's bone, range, radius and "spent" bit (KH2's reaction, group and clash as before);
  - the row bonuses are the left Keyblade's own row;
  - its `+0x90` is `leftblade`'s owner copy (`leftblade::left_owner_handle`), so KH1's own builder lays KH1's sphere row along the left blade, scaled by that Keyblade. KH1 also runs its own sweep and wall test.

  Our KH2 weapon-group spheres and sweep for it are gone; the entry hook only refreshes `+0x90` (`lefthits::before_build`). KH2's windows, ATKPs and re-hits stay. No new hooks. Assumptions: the owner copy passes the pass's actor checks as Sora does; left hits come off the right Keyblade when the left isn't drawn (logged). Research and plan for the second Keyblade as a real KH1 weapon object (pool `0x14296B1E0`, maker `0x140286D60`, model queue `0x1401D4070`, trail `0x1402C2310`): §7.3. Not checked in game. docs/kh1-drive-forms-native.md §7.2, §7.3.

### 2026-10-26: Stage 682 (testing branch: Valor's left Keyblade hits in their own module)
- **Valor's left Keyblade hits in their own module.** The tester (what-is-still-missing, after m13): "for 2nd keyblade it should behave 1:1 with how the main keyblade works, make sure its a file as well". t680's left-Keyblade hit code moves out of `bodyhits.rs` into `win/lefthits.rs`, with its own state (object, windows, open window). `bodyhits` keeps the pass hook `0x1402B3800`, the pool helpers and the data copies. It calls `lefthits::new_play` on a new play and delegates `atkp` / `hit` / the entry build; `takeover` calls `lefthits::step`. No behaviour change. docs/kh1-drive-forms-native.md §7.2.

### 2026-10-26: Stage 681 (testing branch: Base Sora's Square moves by KH2's exe rule)
- **Base Sora's Square moves by KH2's rule.** The tester (what-is-still-missing, after m13): "3. (a)".
  Upper Slash, Horizontal Slash and Finishing Leap now go through the same path as Valor's Square
  moves (`Character::would_take_square_exe`, Steam KH2 `0x1403A81F0` / `0x1403A9950`, Checked
  (decomp)), on KH1's states (`combomods::control`) and on the controller's own action button. The
  count is the one before the press (stage 177 read it after: Upper Slash / Horizontal Slash were
  refused one swing early, Finishing Leap, offset 1 per PTYA moveset 1, Checked (files), came one
  swing early). Added from the decomp: the free-state step of `0x1403D6CA0` (a new combo) and
  Berserk Charge while MP charges (no Square finisher, steps at any count; bit 11 of `+0x1DC` as
  Berserk Charge: Assumption). With the combo style KH1, KH1's combo window stands for KH2's
  (Assumption). docs/kh1-drive-forms-native.md §7.1.1.

### 2026-10-26: Stage 680 (testing branch: Valor's left Keyblade hits as its own KH1 attack object)
- **Valor's left Keyblade hits as its own KH1 attack object.** The tester (what-is-still-missing, after m12): "3. do both", "make sure when your moving these over to be native that you double check everything ... check the kh2 c decomp steam". KH2 keeps one attack per (group, shape-source object) (Steam KH2 `0x14040D810` -> `0x1403DAFE0` -> `0x1403D29B0` -> `0x1403D25D0`, a new attack `0x14040F1C0`; Checked (decomp)), so the two Keyblades always run separate attacks and both may hit the same foe in a swing. The old system put the left windows on KH1's group 0 (the right blade's sphere), `window_atkp` took the first weapon > 0 window (left hits lost where the windows overlap, 154 / 184), and `keyblade_rearm` re-armed the right blade on left windows. Now `kh1_tracks` gives group 0 to weapon 1 windows only and the weapon 2 windows play on a KH1 attack group of their own (`kh2slots::left_channel`, 1 for every Valor move; Checked (files)). For each play `bodyhits::make_left` makes one KH1 attack object with KH1's constructor `0x1402B2DC0` (bone 0x30, motion-tied), on copies of his swing row's data per window, with the left Keyblade's weapon row bonuses as `0x1402B3270` adds them (Checked (decomp)); `bodyhits::left_step` re-arms it with KH1's re-arm at each new KH2 attack (`rehit::new_attack`). t638's entry hook (`0x1402B3800`) puts in the open window's group of KH2's weapon spheres (`W_EX010.mdlx` bone 2, Checked (files)) with KH2's sweep, along the left Keyblade as drawn (grip 280 to blade point 0x30, KH1's `0x14029EB20` on `leftblade`'s owner copy); KH1 does the collision, contact, damage, spark and clash. No new hooks. Assumptions: KH2's distances laid unscaled along KH1's blade; the right Keyblade used when the left isn't drawn (logged). Open: the right blade still uses KH1's row sphere, not KH2's group spheres. Not checked in game. docs/kh1-drive-forms-native.md §7.2, docs/kh1-rehit.md.

### 2026-10-26: Stage 679 (testing branch: Valor Form's Square moves on KH1's rows)
- **Valor Form's Square moves on KH1's rows, as KH2's exe reaches them.** The tester (what-is-still-missing, after m12): "3. do both", "make sure when your moving these over to be native that you double check everything ... check the kh2 c decomp steam". KH2's action-button handler (Steam KH2 `0x1403A81F0`) tries the Square combo rows 0x25, 0x0C, 0x26 through `0x1403A9950`, then row 11 through `0x1403A9C70` with the stick at rest and again after the roll / Quick Run checks; Valor has neither, so row 11 comes out standing or moving (Checked (decomp)). `0x1403A9950` takes a non-finisher row only in the combo window, with the count before the press under the length, the swing connected, never in the finisher window; `0x1403A9C70` takes row 11 only from a free state (`0x1403BFB30`) (Checked (decomp)). Valor's PTYA (moveset 2): row 11 is 181 with a hop of 90 (16..24); rows 12 / 38 are 155 / 183, ground / air, not finishers; no row 37 (Checked (files)). The old controller path took 12 / 38 as finishers (`square_finishers`) and gave Valor Sora's Finishing Leap (s214's `port_moves`), neither KH2's. Now `combomods::valor_square` (from `shared::control` in `form::native_now`) reads KH2's combo / finisher window at KH1's time in Valor's slot and `Character::would_take_square_exe` (a read-only check in the exe's order on KH1's count, connected flag or Combo Master, and `FINISHERS`, row 37 skipped); the row plays on our combo row of its motion (155 / 183, Valor's slots) through KH1's own Attack step. Free on the ground (KH1 state 0, can-act), Square plays row 11 on our "181" row with KH2's hop on KH1's air state (`ours_combo_run`). KH1 still sees no Square in Valor; no new hooks, no rebake. Assumptions: KH1's can-act stands for KH2's free state; Combo Master counts as connected. Not checked: Valor's combo length (`+0x200` / `+0x201`, script-set). Found, not changed: base Sora's Square step rule (stage 177) is one swing stricter than the exe's. Not checked in game. docs/kh1-drive-forms-native.md §7.1.

### 2026-10-26: Stage 678 (testing branch: Valor Form on KH1's own states)
- **Valor Form on KH1's own states.** The tester (what-is-still-missing, after m11): "I want you to transfer drive forms as well", "go ahead". First step: Valor; its moves no longer run on KH2's controller.
- Valor's motions from `P_EX100_BTLF` are baked as KH1 slots of their own with Valor's rig and root travel (`kh2slots::VALOR_SPECS`, `pose::Baker::poses_of`, `pose::Src`), on KH1's requests for idle, walk, run, the jump and High Jump takeoffs (both 201: KH2's jump start plays 201 with High Jump, Steam KH2 `0x140422630`, Checked (decomp)), fall, landing, reactions 10..15, recoils 41 / 43, item use 9 and the ground and air combo, plus our combo rows and Finishing Plus rows. Their KH2 frame triggers ride as KH1 motion events (t677).
- In Valor (`form::native_now`) `kh2slots::redirect` points those requests at Valor's slots whatever the styles; clips of the slot playing come from Valor's moveset (`clip_of`, `pose::kh2_moves`), so turn-to-lock, edge stop, no-gravity, Finishing Plus, guard / invulnerable windows, re-arm and ATKPs use Valor's data.
- KH2's pick in KH1's picker reads Valor's PTYA (moveset 2), KH1's ability finishers held back. Physics on KH1's states are Valor's (`prty` moveset 2: walk 2, run 12, jump 235, gravity 20/49, fall cap 16; High Jump MAX 585) (Checked (files)).
- The Drive is our KH1 request for motion 52 (`kh1rows` "Drive") with KH1's action flag; its no-gravity, invulnerable and hit ranges work through KH1's states (Checked (files)). After 39 frames the tables switch and Valor's arrival 53 plays (`kh1rows` "Valor arrival"). Revert switches back and cuts into KH1's idle or fall (`form::native_frame` from `takeover::shared_frame`, no borrow). `shared::on()` now excludes only forms still on KH2's controller (`form::driver_form`).
- Unchanged: MP cost / drain, KO revert, look, second Keyblade's stats. Slot cache keys and manifest cover Valor's files (one rebake).
- Not native yet: the left Keyblade's hits (its windows use KH1's one sphere on the right blade), its draw (`leftblade`, now fed from the baked frame), Valor's Square moves (KH1 sees no Square in Valor), Valor's guard windows. Assumptions: KH1's can-act or combo window stands for KH2's `can_change_form`; the arrival isn't held in the air; 39 frames is the tester's earlier change length. Not checked: `W_EX010_BTLF_R`, in game. Wisdom and Final unchanged. docs/kh1-drive-forms-native.md §7.

### 2026-10-26: Stage 677 (testing branch: KH2's frame triggers as KH1 motion events)
- **KH2's frame triggers as KH1 motion events.** The tester (what-is-still-missing, after m11): "I want you to implement everything thats not native into being native inside of kh1's engine ... go ahead" (item 6: KH2's sounds, voices and effects on baked moves started by our per-frame tick).
- Each KH2 sound (trigger 8), voice (13 / 14) and effect element (1) of a slot's clip is a type-6 key of the baked motion's own event list, in the template's first sound group (KH1 reads only that one): a = 0x81 / 0x82 / 0x80, b = the KH2 motion, c = the value, time = KH2's frame − 1/64 (frame 0 stays 0) (`kh2slots::kh2_trigger_keys`, `kh1_data::slot::add_sound_keys`).
- KH1's edge pass (Steam `0x14029FB30` from `0x14029F0A0`) collects type 4 / 6 keys with `from <= t < to` (`0x140394900`, 8 a call), holds them through a blend, and runs `0x140295420 (actor, a, b, c)` for type 6. Checked (decomp). That handler is wrapped (Steam only, prologue `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20`, Checked (exe)): our codes on Sora go to `kh2sound::event` / `kh2fx::event`; KH1's own codes (0, 0x19, 1..0x1D, table `0x140528A87`) go on to KH1's handler.
- KH2 fires a trigger at F with `from < F <= to` (Steam KH2 `0x14040E960` from `0x1403C6740` / `0x1403C8A40`) and holds them through a blend (Checked (decomp)): same update as KH2 for whole-frame steps; frame-0 triggers now wait for the blend's end, as KH2's do. Keys past KH1's 8-per-call buffer within 2 frames are left out and logged. KH1 skips type-6 keys while `0x142867364` bit 0x400 is set (`0x1401A22B0`); ours with them (Assumption: events / cutscenes).
- `kh2slots::tick` now only notes which of ours plays and expires forced picks; off Steam the old per-frame reading stays. Slot cache `BAKE_FORMAT` 2 (one rebake). KH2's footsteps (trigger 2) not written: baked slots keep KH1's template foot events (asked). Not checked in game. docs/kh1-anim-baking.md §13.

### 2026-10-26: Stage 676 (testing branch: Aerial Finish holds its foe)
- **Aerial Finish holds its foe: KH2's blow sign, and KH1's scripted launch.** The tester (kh2s-combo-row-selection, m34): "aerial finish is working how it should hit wise, just need to fix the knock back so the enemy doesn't get sent so far off of the first hit, please look into how kh2 does it in the c decomp". KH2's blow (Steam KH2 `0x1403D2FE0` group 4, speeds from `0x1403CEA00`) lays the ATKP's sideways number along the flattened hit direction as a signed value (Checked (decomp)). Aerial Finish's spins 915 / 916 / 917 are reaction 9 with up 10 / 5 / 12 and sideways -1 / -1 / -10, and 927 is 3 / 45 (Checked (files)): the spins pull the foe in, only the last hit throws it. `Launch::kh2` took the size, so 917 threw it 10 a frame away; the sign is now kept. Separately, KH1's enemies of actor kinds 4 / 5 / 9 / 10 don't run the hit pass `0x1402B8E10`: their think `0x1402C2C90` hands each record to their script, which launches through the script call `0x1402BBC80` -> `0x1402B79F0 (actor, record, motion base, height XMM3, speed, down time)` with its own numbers (Checked (decomp + exe)). It wasn't hooked, so KH2's numbers never reached them (the tester's log: 16 launches asked, no launch-hook line). Now wrapped (Steam only, prologue `48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 40`, Checked (exe)): KH1's runs, then for a fresh KH2 launch that launched, KH2's arc and blow flight, shared with `0x1402B8530`'s hook (`combat::kh2_launch`). Explosion's ticks (911: 3 / -1) now pull slightly too, as in KH2. Assumption: these scripts pick the scripted launch for a kind-5 record (log: "KH1's scripted launch on ..."); such an enemy's air step is KH1's state 2. Not checked in game. docs/kh1-launch.md §4.

### 2026-10-26: Stage 675 (testing branch: Finishing Leap leaves the ground)
- **Finishing Leap leaves the ground.** The owner: "im not able to do the finishing leap ability", "the bug still appears on finishers implement a fix". Their trace (Base Combo KH1, Combo Master on): Square in the combo window at f6071 picked Finishing Leap (request 0x201, KH2 motion 167) for one frame, then KH1 played its landing (request 7) at f6073. The row enters KH1's air state from the ground with a hop of height 0 and leaves the floor by its root only from its no-gravity range (range id 3 from frame 1, Checked (files) `P_EX100.mset`), so on the frames before it (the queued request, frame 0) KH1's air step put him straight down and landed him. KH2 starts it from the ground action with no fall there (Steam KH2 `0x1403A9950`, Checked (decomp)). `kh2slots::rise_start`: for up to 4 game frames after such a row starts (`combomods::ours_run` → `start_rise`), until its own range holds him, KH1's air step holds him as in the range and moves him up by the range's first root step (`shared::physics::air_step`). **Assumption**: the lift is at most a frame early; not checked in game.

### 2026-10-26: Stage 674 (testing branch: One KH2 re-hit path for every KH2 move)
- **One KH2 re-hit path for every KH2 move.** The tester (kh2s-combo-row-selection, after m33): "if other moves are multi hits in kh2 and don't have it right now (aerial finish) then yes apply it, I also noticed that explosion was working before this implementation so see if we are not writing two codes for the same thing. check it behind kh2 c decomp". KH2 (Steam, Checked (decomp)): each frame an attack range is open, `0x14040D810` -> `0x1403DAFE0` -> `0x1403D25D0` asserts its ATKP on the group's attack slot (8 per object at `+0xC10`); the same ATKP keeps the attack, another ends it (`0x1403D0EB0`) and makes a new one (`0x14040F1C0`), and an attack whose group is no longer asserted ends (`0x14040F610`). Within an attack a target can be hit again after the row's interval (OG rewrite `HitRecord`, Checked (code)). Before, `bodyhits` kept its own per-target interval (why Explosion re-hit) and t673 re-armed the Keyblade object per window index, so Keyblade windows with an interval (Aerial Sweep 909/10, Aerial Spiral 1131/12, Aerial Dive 1882/8, Retaliating Slash 1582/10) hit once. Now `rehit.rs`: `on_hit` (from `probe::hit`) notes each hit's row interval (`kh1rows::kh2_atkp`), `tick` (top of `takeover::physics`) clears the target's bit once it has run, `new_attack` re-arms with KH1's own `0x1402B3380` when `kh2slots::keyblade_rearm` sees KH2's rule (ATKP or group change, or a window after a gap); `bodyhits`' copy is gone. Per-move table in `docs/kh1-rehit.md` (Checked (files)). Not checked in game.

### 2026-10-26: Stage 673 (testing branch: 153's second slash hits)
- **153's second slash hits.** The tester (kh2s-combo-row-selection, after m32): "the 2nd hit is also missing its 2nd hitbox, its just one for now". KH2's 153 has two Keyblade hit windows (ATKP 128 at 20..32, 147 at 38..47; Checked (files)), and KH2 keeps a foe's hit-once per window (OG rewrite's `HitRecord`, `(swing << 8) | window`; Assumption for the exe). KH1's row has one attack object for the whole motion, whose hit bit a foe keeps, so the second slash missed any foe the first hit (his log: one of 128 / 147 per foe). `kh2slots::keyblade_rearm` (each frame KH1 has Sora) re-arms the row's attack objects with KH1's own re-arm (Steam `0x1402B3380`, Checked (decomp); interval made due for the call, then restored) when a new Keyblade window of the same motion opens; this also applies to ability moves with several Keyblade windows (Dodge Slash, Slide Dash). Log: "KH2's next hit window on the Keyblade ... re-armed". Not checked in game. Notes: `docs/kh1-rehit.md`.

### 2026-10-26: Stage 672 (testing branch: KH2's turn: the turn frame's step follows the new facing)
- **KH2's turn in a swing: the turn frame's step follows the new facing.** The tester (kh2s-combo-row-selection, after m32): "I want you to check into the kh2 c decomp and implement the changes from your findings". In KH2 (Steam, Checked (decomp)) the motion update `0x1403C6740` samples the frame's root step (`0x1403C8520`, motion `+0xFC`) before its ranges turn him (`0x14040E740` -> `0x1403D4B10`), and the mover `0x1403B89A0` / `0x1403B7CA0` lays the step along his yaw `+0x68C` after that turn. KH1's step for that frame (`+0x200`) was made before our turn, so `kh2slots::turn_to_lock` now turns it by the same angle (travel flag `+0x160` & 4), as it already turned the travel so far (`+0x210`, t669). Not checked in game. Notes: `docs/kh1-anim-slots.md`.

### 2026-10-26: Stage 671 (testing branch: Trace: the CSV header matches its rows)
- **Trace: the CSV header matches its rows.** The tester (kh2s-combo-row-selection, after m32, answer 2 (a)). The config columns' header lacked `cfg_magic_combo` (the last move style, `styles::Kind::EVERY`), so from `pos_x` on each value sat under the previous column's name. Added, with a compile-time check that the config header names as many columns as `config_now` writes. Debug only. Checked (code).

### 2026-10-26: Stage 670 (testing branch: Flash Step and Vicinity Break on rows of their own again)
- **Flash Step and Vicinity Break play again on the KH2 combo setting.** The tester (kh2s-combo-row-selection, after m31): "sometimes you can't attack it just resets you". His log f25752: "KH2's row 51 motion 170 has no KH1 row of its own built: no attack". Since t631 neither Flash Step (169) nor Vicinity Break (170) had a slot: `kh2slots::blade_swing` / `body_strike` treated their Keyblade guard windows (`W_EX010.mset`: 169 ATKP 162 at 17..22 and 161 at 22..30; 170 ATKP 162 at 14..32) as not native, so KH2's pick of them did nothing. Keyblade guards are already KH1's hit pass's (`keyblade_guard` in `hurt`, `recoil::guard_window_on`), so they no longer keep a move off our rows. Checked (files): the rule now passes exactly 169 and 170 more. Checked (log): no build since t631 listed them. Not checked in game. Notes in `docs/kh1-anim-slots.md`.

### 2026-10-26: Stage 669 (testing branch: KH2's turn in a swing no longer throws Sora around)
- **KH2's turn in a swing no longer throws Sora around.** The tester (kh2s-combo-row-selection, after m31): "the 2nd attack on kh2 soras ground combos has really crazy tracking and he teleports everywhere". His trace f23068: 153's second turn window (frames 32..44) moved him 70-160 units a frame (23027: (71, -92) on a 0.52 turn). KH1's motion pass (Steam `0x14029F0A0`) re-applies the motion's whole travel so far (H10, kept in `+0x210`) with his current facing, so a turn mid-motion swung all of it round the origin. Also KH1's think (`0x1402A3300` -> `0x1402A8300`) turned him back at pi/16 a frame toward `+0x150`, which the ground input step (`0x1402CC1A0`, `0x140297FB0`) resets every frame from the move vector `+0x20` (the swing's start direction; `0x1402A43A0` / `0x1402A5F10`). `kh2slots::turn_to_lock` now turns `+0x210` by the same angle while the travel flag (`+0x160` & 4) is set, and on the ground sets `+0x20`'s direction to the new facing (its y and speed `+0x2C` kept). Checked (trace, decomp); KH2's per-frame root step from the OG rewrite. Assumption: `+0x20` left alone in the air; the frame's own step (`+0x200`) still follows the old facing for one frame. Not checked in game. Notes in `docs/kh1-anim-slots.md`.

### 2026-10-26: Stage 668 (testing branch: Drive Form recolours built once into the mod folder)
- **Drive Form recolours built once into the mod folder.** The tester (ram-use-and-whats-being-optimized, after m3): "do all three ... go ahead". Each recoloured slot and look `tint` makes (Wisdom's blue on `-1c0` / `-1c1` / `-1c3`, Valor's red on `-1c1` / `-1c3`) is written once to `kh1-kh2-cache\form-tint-<look>-1c<n>.bin` beside the DLL (`tintcache`, the slot cache's file layout through `slotcache::write_named_blocks` / `decode_shared`) and loaded from there in later sessions. The manifest holds the tint format, VERSION, game build, look, slot, the source's name, size and content hash, and its origin. A file is used when KH1's in-memory DDS (`+0x68`) hashes the same, or, with no copy in memory, when it was made from the PKG and KH1's PKG entry (offset, lengths, `.pkg` size and time) is unchanged, so the PKG isn't read; anything else is rebuilt. Filled lazily on a thread at a form's first need: nothing at start (the loader can't use KH1's copy; Sora isn't loaded before the title). The recolour moved off the game thread (it ran there when KH1's copy was in memory), so KH1's colours show for a few frames on first use. Pixels dropped once the texture is made (t660's memory kept). Checked (code; tests `tintcache::tests`). Assumption: timings (tens of ms on a thread). Not run in game. Docs: `docs/kh1-asset-cache.md` §6, `docs/kh1-tint.md`.

### 2026-10-26: Stage 667 (testing branch: KH2 effect packages built once into the mod folder)
- **KH2 effect packages built once into the mod folder.** The tester (ram-use-and-whats-being-optimized, after m3): "do all three ... go ahead". Each KH2 effect package made into a KH1 DPX (`kh2dpx::build`: p_ex100, w_ex010, the forms', the spells') is saved once as `kh1-kh2-cache\kh2-effects-<label>.bin` beside the DLL (`dpxcache.rs`, `kh2dpx::cached_or_built`): a manifest, the DPX as built (before KH1 relocates it), its MD5, its own HD images (each once), each record's image, and the layers, program types, KH2 program places, lookup entries and palettes. It's read straight into the DPX's pages (no extra copy), with the images in one buffer freed once the package is entered. Rebuilt when the mod version, the game build, the KH2 copy (backup or live), what the build is given (Sora's `kh2_effects` sets included), the package's stored bytes in its `.pkg` (`kh2_formats::egs::Archive::read_stored`, `kh2_data::Install::read_stored_any`, hashed) or `texcommon.dpd` changes; a damaged file is rebuilt. Packages are still loaded only when they can play and freed when they can't; `texcommon`'s images are still read at every start. A lower start-up peak rather than a shorter start (builds were 0.01-0.08 s). Checked (code; unit tests). Assumption: a `.pkg` entry's `stored_len` covers its HD assets; timings. Not run in game. Doc: `docs/kh1-asset-cache.md` §5.

### 2026-10-26: Stage 666 (testing branch: KH2 sounds built once into the mod folder)
- **KH2 sounds built once into the mod folder.** The tester (ram-use-and-whats-being-optimized, after m3): "do all three ... go ahead" (the assets built once from the player's own files into the mod folder). The KH2 sounds cut from KH2's banks are kept in `kh1-kh2-cache\kh2-sounds.bin` beside the motion slot cache: ~104 one-sound banks in KH1's form, which element / sound id / spell element plays which, Sora's voice table rows and his battle voice bank (`soundcache.rs`). Later starts read only that file, each sound streamed into the buffer `kh1bank` takes over (one copy each); the effect package, the three KH2 banks (~28 MB), `00battle.bin` and the voice bank are no longer read. The manifest holds the format, VERSION, `kh2_volume`, a hash of what is wanted (`kh2_sounds`, Reflect, forms, spells) and a stamp of each KH2 file the cut read (`kh2_data::Install::stamp_any`: archive, `.hed` entry, `.pkg` size and time), so a change is seen without reading the files. Anything different, or a damaged file: cut again (now one KH2 bank at a time) and rewritten (temporary file, then rename). The loading screen holds KH1's title for it only when it is cut again ("Building KH2 sounds (n of N)", `loadscreen::job`). Checked (code), unit tests. Assumption: anything changing a file in a `.pkg` changes its index entry or the `.pkg`'s size or time; RAM and time figures estimated. Not run in game. Doc: `docs/kh1-asset-cache.md`.

### 2026-10-26: Stage 665 (testing branch: Debug menu: Growth Abilities page)
- **Debug menu: a Growth Abilities page.** The tester (quick-run-native, after m6): "can you make it a setting in the debug where I can adjust the growth abilities", then "the overrides can always be adjusted in the debug, keep it", "don't worry about drive forms", "(a)", "this is purely for testing, the debug menu should be able to break any rule". A Growth Abilities page with rows for High Jump, Quick Run, Dodge Roll, Aerial Dodge and Glide: Normal | Off | LV1 | LV2 | LV3 | MAX (left / right step, confirm back to Normal, the level the controller has now shown). Saved in the ini (`debug_high_jump`, `debug_quick_run`, `debug_dodge_roll`, `debug_aerial_dodge`, `debug_glide`), read only with `debug_menu = on`; without it nothing is overridden. `growth::with_growth_override` replaces the five levels where `menu::Growth` hands KH2's controller its abilities, so every reader follows: the controller's fmab records, the native Quick Run state (speed, brake, timer, invulnerable frames, the LV2+ start effect), the roll's invulnerable frames, Aerial Dodge, the Square routing, the High Jump height on KH1's states, the glide level kept for KH1's glide style (`takeover`), and the menu names. A forced level is given even if not learned or equipped; Off removes it. Not covered: KH1's own jump / roll / glide (style rows on KH1), which read KH1's ability list. Drive Forms unchanged (a form's own growth set; Sora's comes back on revert). Nothing written to the save. Checked (code); test `growth_override_replaces_levels`. Not run in game. Doc: `docs/kh1-debug-menu.md`.

### 2026-10-26: Stage 664 (testing branch: Log categories on the debug menu, all off)
- **Log categories on the debug menu's Logs page, all off; no log at all without the debug tools.** The tester (ram-use-and-whats-being-optimized, m1 answer 3): "have our logs attached to the debug menu where you can turn on and off each log and all should be off, if debug menu is off then none of the logs are injected into the game". Every log line is now `log!(Category, ...)` (`win/mod.rs`, `LogCat`); the macro checks `log_on` (one atomic load) before formatting anything, the per-module `note` / `cmd_log` helpers take closures, and the "first N lines" counters count only lines written. A new Logs page in the debug menu (All Logs plus 14 categories: General, Loading & Cache, Takeover & Input, Animation & Pose, Combat & Hits, Magic, Drive Forms, KH2 Sounds, KH1 Sound Calls, Effects, Camera, Menus & Config, Debug Tools, Crash & Version), saved as `log_*` in the ini, all off by default. KH1 Sound Calls replaces the `sound_log` key. Without the debug tools no `kh1-kh2.log` is created, old logs aren't rotated and the crash handler isn't installed (the literal reading; `CRASH_NOTES_ALWAYS` flips it, asked in the thread). The trace's log lines and sounds only see enabled categories. README says how to get a log. Checked (code); not run in game. Doc: `docs/kh1-debug-menu.md`.

### 2026-10-26: Stage 663 (testing branch: Debug tools loaded only with debug_menu = on)
- **Debug tools loaded only with the ini's `debug_menu = on`.** The tester (ram-use-and-whats-being-optimized, m1 answer 3): "in the ini I want to make it where if debug menu is off then we don't inject or load anything related to the debug, if its on then we do. keep debug menu in the config menu but if its off in the ini then its gone from that menu." `debug_menu` is read once at start, before any hook goes in (`win::debug_on`); a later ini save doesn't add or remove hooks. With it off, none of these are installed or kept: the debug menu's draw hook (KH1's end of frame drawing, Steam `0x1400FBBE0`), so the overlay's 4 MB vertex ring, font buffer and font are never made and the Frame Advance panel isn't drawn; Frame Advance's keyboard hook and hold; the menu's input and stat locks; the trace recorder (no rows, no dumps); the five log-only sound-log hooks; the heartbeat thread (with the Insert test key and the event-opcode trace); and the probes' debug-only work (actor dumps, position search, `hit_effect_note`, the swing-record template). Kept, as gameplay runs through them: KH1's frame end (Steam `0x140109B40`) for `kh1bank::frame_end`, the voice group hook `0x14028E060` (KH2's hurt line and quotes), and every `probe.rs` hook (set-motion counts, hits, pad merge, event opcode 322 for the attack tutorial). Config > Misc drops its Debug Menu row (and its help mention) when the ini has it off; with it on, the row switches the menu for that session only. Checked (code): what each kept hook carries. Not run in game. Doc: `docs/kh1-debug-menu.md`.

### 2026-10-26: Stage 662 (testing branch: Slot cache: the same bake asked twice is one block)
- **Slot cache: the same bake asked twice is one block.** The tester (ram-use-and-whats-being-optimized, item 4.7, with m1 answer 2 (a)). Of the bakes, 7 are byte-identical repeats (Finishing Plus's next finishers 154, 155, 166, 183, 184, 194, 197 on a second request of ours; 1.9 MB; Checked (log)). With t655's cache the first ask took the file's copy, so those 7 were re-baked and the file rewritten every start (Checked (code)). Now a repeated key returns the same block (test `same_bake_twice_is_one_block`); log "N the same bake again (shared)". KH1's placed copies stay one per slot: KH1's pose pass `0x140391C70` writes the sampled helper channels into the motion's own helper records (`+0x30`) every pass (Checked (decomp)) and KH1's own `xa_ex_0010.mset` never shares a motion between slots (Checked (files)). Doc: `docs/kh1-asset-cache.md`.

### 2026-10-26: Stage 661 (testing branch: Start-up: package key and indexes read once)
- **Start-up: the package key and indexes are read once.** The tester (ram-use-and-whats-being-optimized, m1 answer 2 (a)). Every package read used to read the whole executable again (KH2's 8.7 MB, KH1's 7.5 MB), scan it for the key and re-read the `.hed` index (Checked (code)). `kh2_formats::egs` gains `KeyTable::from_executable_cached` and `Archive::open_cached`: one per path for the run, read again only if the file's size or time changes; the lock is held while one is read, so loader threads starting together wait instead of each reading it. `kh1_data::Kh1Install` and `kh2_data::Install` use them. A smaller, shorter start-up peak (the read count of ~100 is an Assumption). Test `index_read_once_until_it_changes`. Not checked: start-up time in game. Doc: `docs/kh1-asset-cache.md` §3.

### 2026-10-26: Stage 660 (testing branch: Drive Form recolour: Sora's textures read only when needed)
- **Drive Form recolour: Sora's textures read only when needed; one recoloured texture per slot and look.** The tester (ram-use-and-whats-being-optimized, m1 answer 2 (a)). Nothing is read at start any more (before, a thread kept Sora's three decoded HD textures, ~10.5 MB, for the whole session; Checked (code)). When a form needs a slot and KH1's own DDS at `+0x68` doesn't fit, that one slot is read from the PKG on a thread (`tint::pkg_pixels`; KH1's colours show meanwhile), recoloured in place and dropped once the GPU texture is made; with KH1's copy in memory the PKG isn't read. Recoloured textures are kept per slot and look and reused for a reloaded model's source with the same name and size: before, each area change made new ones and none was released (Checked (code), a leak). Exception: a source that went away while holding ours isn't reused (Assumption: KH1 may release it). Not checked: in game, how often KH1's own copy is there. Doc: `docs/kh1-tint.md`.

### 2026-10-26: Stage 659 (testing branch: KH2 player model not loaded while it's off)
- **The KH2 player model isn't loaded while it's off.** The tester (ram-use-and-whats-being-optimized, m1 answer 2 (a)). `model::load` returns at once unless `model::on()` holds, logging "KH2 model: not loaded (Player Model off)"; `on()` has been false since s259. Every reader (`hide`, `hide_menu`, `draw`, `is_menu_copy`, `left_is_ours`, the Keyblade loader) checks `on()` first and treats the model as missing (Checked (code)). About 3 MB less (4 bodies × ~0.8 MB, measured on P_EX100.mdlx) and one loader thread fewer at start. Not checked: in game. Doc: `docs/kh1-kh2-model.md`.

### 2026-10-26: Stage 658 (testing branch: Baked motions held once)
- **Baked motions held once.** The tester (ram-use-and-whats-being-optimized, m1 answer 2 (a)): "do them now as small separate versions ... if the agent isn't removing the duplicates in the asset work then tell it too". While loading, up to four copies of the bake were held (the cache file's bytes, each block copied out of it, the cache's write-back clone, the build's `Built.blocks`), and `Built.blocks` stayed for good after placing. `slotcache::Block` is now a shared buffer: a cached block is a slice of the file's one buffer, a fresh bake its own; the cache's write-back list and `kh2slots`' build share them. `Cache::open` takes the file's bytes by value; `Cache::write` streams the file out with the check hashed as it goes. `kh2slots::place` (runs once; `PLACED` is never reset) takes `Built.blocks` and frees each block once copied where KH1 plays it from; nothing reads them after (Checked (code)). About 16-18 MB less after placing (block sizes from the logs' "baked (… bytes" lines), a lower start-up peak. Not checked: Task Manager numbers. Doc: `docs/kh1-asset-cache.md`.

### 2026-10-26: Stage 657 (testing branch: Native loading screen: KH1 waits before its title while the mod builds)
- **Native loading screen: KH1 waits before its title while the mod builds from your files.** The tester (what-is-still-missing, m8 answer 2 (a)): "when the game starts there should be a loading screen showing what the extraction is doing before continuing into the menu, this should render in engine and be native, and not as a layer like the debug menu", shown only when something is being rebuilt. `loadscreen` redirects KH1's mode-machine call at `0x140170E1A` (-> `0x140170870`) and skips it while the state at `0x14233FBC0` is 2 (the state that loads `dktitle.rel`) until the loader thread (`pose::load_kh2`) ends; a guard covers a failure or a panic, with a 90 s cap. The rest of KH1's frame runs on. Once a bake really runs (a slot cache miss), the progress lines ("Building KH2 move M (n of N)", "Saving the built moves") are drawn after KH1's 2D work (call `0x140170FF0` -> `0x14017B190`) with KH1's text routine `0x1402CFBF0 (align 2, colour, x, y, text, scale)` in KH1's font. On release both call sites get KH1's bytes back: nothing of ours runs per frame. Checked (exe): call bytes and the state address. Checked (decomp): no other reader of state 2, the quit check and present still run, no movie before the title, the font table set up in state 1, the text routine's arguments (the old plan had them in the wrong order). Assumption: the font texture is ready in state 2, held frames present black. Not checked in game. Steam only. docs/kh1-asset-cache.md §2.

### 2026-10-26: Stage 656 (testing branch: Cast from Glide: a KH2 spell ends KH2's glide into KH1's air cast)
- **Cast from Glide: a KH2 spell ends KH2's glide into KH1's air cast.** The tester (what-is-still-missing m8, now thread cast-from-glide): "1. (a) then make it a config in the tweaks config menu". Steam KH2's glide motions 205..208 have free range 1 all through (Checked (files), P_EX100.mset), so its input step `0x1403D6FC0` takes the magic command through `0x1403D6CA0`; the cast start `0x1403C5170`, airborne, replaces the glide with the air cast, and the glide's exit `0x140423B70` / `0x1403CB8A0` leaves the velocity alone (Checked (decomp)). `combat::glide_cast`: a KH2 spell (Magic KH2's Fire..Cure, Reflect) picked in KH2's glide on KH1's state 0x15, whose cast will go ahead (slot and files built, KH2 cost payable, `reflect::air_ready`), ends the glide into KH1's fall state 2 (`glide::end`); KH1's command step `0x1402A3600` then casts its state-2 air cast with the KH2 spell, as from any fall (Checked (decomp)). KH1's think `0x1402A3300` sets `+0x3C0` from the stick on a state-change frame (flag 0x400, state 2) (Checked (decomp)); the cast's velocity is kept over it (`glide::keep_cast_velocity` -> `carry`). New Combat > Tweaks row "Cast from Glide" (Off | On, ini `glide_cast`, default on). Off, KH1's own glide, or a cast that can't go ahead keeps t654's refusal. Assumption: KH1's menu takes commands in state 0x15. Assumption: no frame of the fall request `0x14029F7C0` shows before the cast's `0x14029F980` replaces it. Not checked in game. docs/kh1-glide-native.md §8.14.

### 2026-10-26: Stage 655 (testing branch: Motion slot cache: KH2's baked slots kept in the mod folder)
- **Motion slot cache: KH2's baked slots kept in the mod folder.** The tester (what-is-still-missing, m7 answer 2 (a)): build the assets once from the player's KH1 and KH2 files into the mod folder, nothing to install. The KH1 motion blocks `kh2slots::prepare` bakes are saved to `kh1-kh2-cache\kh2-motion-slots.bin` beside the DLL (`slotcache`, `win::dll_dir` via `GetModuleHandleExW`, game folder as fallback). A manifest names what they're made from: bake format, mod VERSION, game build, hashes of KH1's `xa_ex_0010.mset` / `.mdls` and every KH2 file read for Sora (`kh2_data::Install::load_sora_noting`), and the ini keys `kh1_slots`, `kh1_leg_ik`, `keyblade_turn`; each block carries its own key (template bytes, motion, length, events, travel). On a match the blocks are loaded instead of baked; otherwise baked and the file rewritten atomically (temp + rename). A damaged or unwritable file only means baking as before. Nothing runs per frame. Unit tests in `slotcache.rs`. Checked (code, tests). Not checked in game. docs/kh1-asset-cache.md §1 (the native loading screen is planned in §2, not built).

### 2026-10-26: Stage 654 (testing branch: KH2 spells in states KH1 casts nothing in: refused, no borrow)
- **KH2 spells in states KH1 casts nothing in: refused, no borrow.** The tester (what-is-still-missing, m7 answer 1 (a)): refuse them the way KH2 does in KH2's states and the way KH1 does in KH1-only states, after re-checking KH2's rule on the Steam exe. Steam KH2's input step `0x1403D6FC0` takes a magic command only through `0x1403D6CA0(player, 1)`: the combo or Finishing Plus window, else a free bit `+0x18C & 0x1C`, set by motion ranges 0/1/2 (`0x14040D810`) (Checked (decomp)). Quick Run 203/204 has no such range; the glide 205..208 has range 1 all through, so KH2 does cast out of a glide (the old Epic rule was wrong there) (Checked (files), P_EX100.mset). `combat::kh2_spell_refused` (Magic KH2's Fire..Cure, Reflect): in KH1's Quick Run state 0x2A and glide state 0x15 refused with KH1's "can't" (the glide pending the tester's answer); in KH1-only states Fire..Cure go to KH1's own command step, which casts nothing (`0x1402A3600`, Checked (decomp)), Reflect does nothing. No KH2-controller borrow in any of these. Roll, guard, landing and Aerial Dodge unchanged (KH1's may-act follows KH2's free frame on our slots, Checked (code)). Not checked in game. docs/kh1-magic-native.md §4p.1.

### 2026-10-26: Stage 653 (testing branch: KH2's fall after a hover starts from rest)
- **KH2's fall after a hover starts from rest.** The tester (air-physics thread, after m3, with kh1.mp4 / kh2.mp4): "I cast air magic > air magic keep me suspended in the air > after its done suspending me I fall straight down (not like kh2)". In the videos KH2's air Reflega holds to cast frame 74 then falls from rest, speeding up, landing about 31 frames later; ours dropped at 16 a frame and landed in about 8 (Checked (video)). The cause is KH2's player think, not the jump step: SORA's vtable `0x1405CBA28` slot +0x10 (-> `0x1403B4980`) marks `+0x120` bit `0x800000` while the motion holds him, and slot +0x28 (-> `0x1403DAD10` -> `0x1403B4780`) restarts the arc as a fall at his actual vertical speed `+0xA3C` once the hold is gone (time = speed / g, rise and height 0, base where he is), 0 after a hold (Checked (decomp); vtable by RTTI and the jump action's flags `0x181` at `0x140750960` Checked (exe)). `shared::physics::air_step` now does the same on the first air step after any KH2 hold (air casts, Aerial Finish, Magnet Burst, Aerial Dive, Finishing Leap) at physics KH2 (`HELD_LAST`, `kh2_fall_at`); the clock still runs during the hold as `0x1403CB360` does. Assumption: the restart speed is his last frame's move. Not checked in game; Drive Forms not covered (KH2's controller). docs/kh1-magic-native.md §4n correction.

### 2026-10-26: Stage 652 (testing branch: Glide bank to bank: KH1's chained blend from the pose on screen)
- **Glide bank to bank: KH1's chained blend from the pose on screen.** The tester (kh2-glide-in-kh1s-glide-state, after m8: "sora now disappears and gets all garbled when turning in glide"; m9 answer "1. (b)": straight to the proper fix). t646 forced KH1's fresh blend start (Steam `0x140394080`) mid-blend by clearing `+0x170`. During a blend that start's source `+0x1B8` holds the blend pass's (`0x140392560`) accumulated matrices for the bone-flag-bit-30 bones only; the other 151 of Sora's 293 bones are unwritten, in a block both starts allocate uncleared (`0x14028DA20`), and the start reads all of it as local matrices (Checked (decomp); the garble inferred from the tester's log: 13 mid-blend fresh starts, every one a rocking bank change). t646 is removed; KH1 keeps its chained start `0x140393B30`. In the glide pre-step a glide motion queued mid-blend (`+0x178` >= 1, `+0x170` > 0) snapshots last frame's drawn pose in the source buffers' formats (bone matrices lerped target x w + source x (1 - w) at `+0x174 / +0x170`, `0x1400E64D0` / `0x1400E7CB0`; the `+0x1B0` channels' first vec4; root offset `+0x100`; the IK goals), and the existing blend-pass hook (`pose::pose_ik`) writes them over `+0x1BC` / `+0x1C4` / `+0x110` / the IK goals' sources on the first pass after KH1's chained start, recognised by the blocks it moved; anything else keeps KH1's own start (logged). Assumption: chains with more than one animated link get the product of lerps (a second-order difference). Glide motions 205..208 only. Not checked in game. docs/kh1-glide-native.md §8.13 (§8.11 replaced).

### 2026-10-26: Stage 651 (testing branch: Gravity, Stop and Aero on KH2's Thunder finisher cast)
- **Gravity, Stop and Aero on KH2's Thunder finisher cast.** The tester (kh2s-combo-row-selection, m30 answer 2): "(a) make them play the thunder finisher animation, also forgot to add aero to the list so also make it the same as gravity and stop". With their animation row at KH2, KH1's ground requests 0x3A / 0x3B / 0x3C now play our slot of KH2's Thunder finisher cast 63 (94 frames, finisher window 42..78; Checked (files), `P_EX100.mset`) instead of Thunder's 62 / Cure's 65 (`kh2slots::CASTS`); KH1's own script still casts the spell, on 63's magic-cast event. Aero is now a finisher like Gravity and Stop (`formmagic::FINISHER_SPELL_REQUESTS`, and out of the Magic Combo row's plain steps), all three with Thunder's finisher cast's Finishing Plus window. Air casts unchanged. Config help for the three rows updated. Not checked in game. docs/kh1-magic-native.md §4c.1e.

### 2026-10-26: Stage 650 (testing branch: An Attack after a cast only when its spell connected)
- **An Attack after a cast only when its spell connected.** The tester (kh2s-combo-row-selection, m30 answer 1 (a)): KH2's rule, the combo goes on from a spell only if it hit (or with Combo Master). Steam KH2's input step `0x1403D6FC0` asks `0x1403D6CA0` with the window test on for the Attack command only with the owner's `+0x120` bit 0x2000000 or Combo Master, for the magic command always; `0x1403D6CA0` clears the flag on a take and the hit routine `0x1403D12C0` sets it on the attack's owner (Checked (decomp)). `formmagic::take_step` now clears KH1's "connected" `+0x50` (was set) and `formmagic::spell_hit` (from `probe::hit`, Sora's free-standing attack objects, `+0x30` bit 0x400) sets it while the cast plays, so KH1's Attack step (`0x1402A7510`) counts on in the cast's window only after a hit or with Combo Master; `note_step` counts a cast on in any window. Not checked in game. docs/kh1-magic-native.md §4c.1d.

### 2026-10-26: Stage 649 (testing branch: KH1's spells in states KH1 casts nothing in: KH1's rule)
- **KH1's spells in states KH1 casts nothing in: KH1's rule.** The tester (what-is-still-missing, m6): "(b) if you have magic set to kh2 those magics should be 1:1, the reverse is true regardless of the animation." KH1's cast `0x1402A3600` casts only in states 0, 6, 0x18, 2 and 8; its menu `0x140293480` refuses state 1, deep water (`0x1402C16E0`) and a failed may-act test `0x1402A2EC0` (Checked (decomp)). Gravity, Stop and Aero with their animation row at KH2 were borrowed by KH2's controller in the other states; now KH1's command step runs them unmodded, casting nothing as KH1 does (`combat::command`, logged "KH1's rule, no borrow"). KH2's spells in those states are unchanged (still a borrow; open question). Not checked in game. docs/kh1-magic-native.md §4p.

### 2026-10-26: Stage 648 (testing branch: KH2's air combo on KH1's flight attack)
- **KH2's air combo on KH1's flight attack.** The tester (what-is-still-missing, m6): "(a) if there's a more elegant implementation to let the animations work as flight air attacks e.g if the game has a different animation for sora when attacking in the flight state then we'll just give kh2 more animations, if not then find how it works and give kh2 the ability". KH1 has no flight attack animation of its own: in flight (state 8) KH1's Attack step `0x1402A7510` runs the air picker's row `0x1402A6C70` + 3 (rows 19 / 20 / 21: requests 0xCE / 0xCD / 0xCC, the air combo's, lunge 8), and the row runner `0x1402A68F0`'s case for 19..21 levels the direction, moves him at the lunge and queues the flight motion 0x75 (Checked (decomp); rows Checked (files), btltbl `+0xA9F0`). At Aerial Base Combo KH2 the air picker hook makes KH2's air pick in state 8 as in state 2; a KH2 air move on a row of ours returns 16 / 18 so KH1 runs its flight row 19 / 21 (`combomods::flight_pick`), and `combat::exec` runs that row with our row copied into its place for the call (request, attack data, trail / effect / voice bytes; KH1's lunge kept; `kh1rows::run_as`, put back after). Anything else stays KH1's own flight row. With shared states on, the old `FLY_ATTACK` borrow was already unreachable (Checked (code)). Assumptions: our requests resolve in Neverland's motion set; KH1's runner reads the row only at its start. Steam only. Not checked in game. docs/kh1-anim-slots.md §7.z.10.

### 2026-10-26: Stage 647 (testing branch: The fall's loop range no longer leaks into the next motion)
- **The fall's loop range no longer leaks into the next motion.** The tester (kh2-glide-in-kh1s-glide-state, after m7): "I also noticed that neutral glide resets its animation wrong when setting jump to kh2 and doing a glide out of jump". The trace (f113654..f113834): the hover 208 (100 frames) looped 13..32 after our baked KH2 fall, the fall's own loop range (Checked (trace)). `kh2slots::loop_range` runs from KH1's pose pass, which KH1's apply (Steam `0x1402A0350`) calls on the incoming motion before writing `+0x168` and after `0x14029FC40` cleared `+0x160` bit 6: it still saw the fall, set its 12..32 and bit 6 again, and a blended start keeps the range it finds (Checked (decomp)). Now it does nothing while `+0x160` bit 0 is set (a request queued or being applied: set by `0x14029F7C0`, cleared by the update `0x14029F0A0` after the queue step, Checked (decomp)); the fall's range is set one frame later. Also covers a 205 glide out of the fall. Not checked in game. docs/kh1-anim-baking.md §7.2, docs/kh1-glide-native.md §8.12.

### 2026-10-26: Stage 646 (testing branch: Glide bank to bank as a fresh blend)
- **Glide: bank to bank as a fresh blend from the pose on screen.** The tester (kh2-glide-in-kh1s-glide-state, after m7): "I'm still seeing the same phenomenon, I sent you a trace ... if you find a fix for both just go ahead and implement it." A bank switch passes his facing (206 → 205 for one frame → 207, Checked (trace)), so the second change came while KH1's blend still ran. KH1's apply (Steam `0x1402A0350`, from `0x14029FC40`) chains such a request (`0x140393B30`): its source becomes the last blend's target, 205's frame 0, not the pose on screen; its fresh path `0x140394080` blends from `+0x1B8` / `+0x1B0`, which the blend pass `0x140392560` keeps as the drawn pose; KH2's `0x1403C6DC0` → `0x1403C86A0` never chains (Checked (decomp) both). `glide::fresh_blend`: when the pre-step's request is queued (`+0x178` ≥ 1) during a blend (`+0x170` > 0), `+0x170` is set to 0 so KH1 takes the fresh path; the motion still starts at frame 0, held through the 16-frame blend. Assumption: KH2's blend source is the drawn pose (the OG rig port, not re-traced). Glide motions 205..208 only. Not checked in game. docs/kh1-glide-native.md §8.11.

### 2026-10-26: Stage 645 (testing branch: Glide out of the Aerial Dodge on KH2's frame)
- **Glide out of the Aerial Dodge on KH2's frame.** The tester (aerial-dodge-native, after m4): "air dodge right now you seem to not be able to cancel it into a glide as fast as you can in kh2, can please check and see against the c steam decomp if its letting you? if you see the reason why then implement it." Steam KH2's glide start `0x1403A81F0` (PLAYER vtable `+0x100`) needs a free bit `+0x18C & 0x1C` (`0x1403BFB30`); 202's only free range is id 1 from frame 4 (Checked (files)). KH2 asks 202 at time 0 in the input step; its motion update `0x1403C6740` skips 0→1 to 2 and runs before the input step (`0x1403BFD30`: update, then PLAYER `+0x20` → `0x1403A89A0`; vtable by RTTI, Checked (exe)), so Square glides 3 frames after the dodge press (Checked (decomp)). Ours was 5 (the tester's traces: the action flag clearing on the fifth frame, Checked (trace)), as KH1 plays 0, 1, 2, 3, 4 and its control step runs before its animation. `combomods::dodge_clock` puts the slot time at 1 on the first frame after KH1 applies our dodge request (KH2's skip, as `glide::kh2_clock`), and the dodge slot's "may be left" starts one frame before KH2's free frame (`kh2slots::INPUT_LEAD`): 3 frames after the press, as KH2. Assumption: KH2's motion step is 1 per 60 Hz frame. Other KH2 slots' free frames keep the one-frame order lag. Not checked in game. docs/kh1-aerial-dodge-native.md §9.

### 2026-10-26: Stage 644 (testing branch: Gravity and Stop as the combo's finisher)
- **Gravity and Stop as the combo's finisher.** The tester (kh2s-combo-row-selection, after m29): "for gravity and stop, there's a phenomenon in kh2 where those magics count as a finisher so the combo row skips straight to the finisher, if you have a finishing plus equipped you can combo into another finisher ... that would be applied for kh1 gravity and stop as well if the config is set to can combo". Ground Gravity (0x3A) / Stop (0x3B), base Sora, on KH2's cast animation always and on KH1's with Magic Combo KH2 (`formmagic::finisher_spell`): the step is the combo's length (or one past it in a Finishing Plus window, `note_finisher_step`), taken as a finisher at KH1's spell start (`take_step`, `combomods::cast_step`); KH1's count 0 after it unless Finishing Plus leaves another finisher. Their KH2 cast slots get no combo window (built as finisher casts); the Attack / spell checks and KH1's combo window take only a Finishing Plus window placed by share from KH2's Thunder finisher cast 63 (Gravity) / Cure's 68 (Stop), range 50 42..78 (Checked (files); placement an Assumption; `finisher_spell_window`, `kh2slots::finisher_share`). KH2 has no Gravity or Stop: the rule is the tester's. Not checked in game. docs/kh1-magic-native.md §4c.4.

### 2026-10-26: Stage 643 (testing branch: Magic Combo: KH1's grounded casts in the combo)
- **Magic Combo: KH1's grounded casts in the combo.** The tester (kh2s-combo-row-selection, after m29): "make a config for magic that allows you to combo out of grounded magic like kh2 ... for kh1 magics logic you just apply kh2s fire combo logic on the first shot and the repeated shot animation ... go ahead". New row Combat > Magic > Magic Combo (KH1 | KH2, ini `magic_combo`, default kh1; `styles::Kind::MagicCombo`). At KH2 (base Sora, ground, no form): KH1's own Fire / Blizzard / Thunder / Cure / Aero cast takes its step of the combo (`formmagic::note_step` at the command, `take_step` at KH1's spell start `0x14028F140`); KH1's own cast animation (requests 0x36, 0x3D, 0x37, 0x38, 0x39, 0x3C, Checked (files)) gets KH1's combo window (instance `+0x14` bit 22) from KH2 Fire 56's share (frame 60, `kh2slots::combo_share`) to its end (`formmagic::kh1_cast_window`, from `kh2slots::frame`), so KH1's menu and Attack step (`0x1402A2EC0`, `0x1402A7510`, Checked (decomp)) take an Attack or a spell. KH1's quick Fire unchanged. Gravity / Stop not yet. Assumptions: the window by share; KH1's script casts whatever motion follows. Per frame: one flag write while a KH1 cast plays. Not checked in game. docs/kh1-magic-native.md §4c.3.

### 2026-10-26: Stage 642 (testing branch: KH2's combo window on its casts, range 35 by motion)
- **KH2's combo window on its casts, range 35 by motion.** The tester (kh2s-combo-row-selection, after m29): "you can combo kh2 magic into each other faster than going from a magic into attack so I need you to look into it because its not 1:1", "if I use reflect ... the move doesn't let you combo", "kh2 blizzard can't combo into anything". Steam KH2's range step `0x14040D810`: range id 35 sets the combo window bit (`+0x18C` 0x400, as range 11) only when its parameter equals the motion playing (the motion player's `+0x28`, set from the queue in `0x1403C6740`), Checked (decomp); Sora's casts carry copies of other casts' ranges (Reflect 71 Fire's, Cure 65 Thunder's), Checked (files). `WindowKind::ComboMagic` now carries its motion (kh2-data); `kh2slots::combo_kind` makes KH1's combo window on our cast slots, and the Attack and spell checks in `combat::command`, KH2's: Fire 56 from 60, Blizzard 59 from 36, Thunder 62 from 42, Reflect 71 34..46, Cure 65 none. Before: spells from every range 35's start (Fire 34, Cure 42, Reflect to its end, Blizzard never), Attacks only in range 11. Drive Forms' controller unchanged (`ComboMagic(_)`). Not checked in game. docs/kh1-magic-native.md §4c.1.

### 2026-10-26: Stage 641 (testing branch: Finishing Plus counted at the KH2 setting)
- **Finishing Plus counted at the KH2 setting.** The tester (kh2s-combo-row-selection, after m28): "if you set combat to kh2 for both settings you can attack out of your finisher indefinitely, it needs to behave exact like kh1". `combomods::finisher_chain_open` returned "always" at the combo style KH2, so with Finishing Plus every KH2 finisher opened KH1's combo window again (`kh2slots::finisher_window`) and chained without end. KH2 counts its finishers in its Attack and command check (Steam KH2 `0x1403D6CA0`: the finisher window only while `+0xD75` is under the maximum, Checked (decomp)), not in its row selection, so both styles now count the same way (`FINISHERS` against one plus Finishing Plus's copies), and a press that still reaches KH1's Attack step in a finisher with none left is no attack (`attack_row`), as at the KH1 setting. No addresses changed. Not checked in game. docs/kh1-anim-slots.md §7.z.9.

### 2026-10-26: Stage 640 (testing branch: KH2 spells' attacks in KH1's own collision pass)
- **KH2 spells' attacks in KH1's own collision pass.** The tester (what-is-still-missing, after m5, "(c)"), first step. Each attack a KH2 spell script makes (Fire, Blizzard, Thunder, Cure, Reflect's counter / burst, Wisdom's shots) is a free-standing KH1 attack object made with KH1's constructor `0x1402B2DC0`, as KH1's spell scripts make theirs (script call 23; first bytes checked, Checked (exe)); KH1's collision pass collides it (t638's entry hook puts in the script's shapes). Its data: the KH1 spell's entry or Sora's swing / finisher row's copy, clash type 6, KH2's spark group and the package's effect list. KH2's hit record kept: the count from the group picks the ATKP row (`bodyhits::before_hit`, `kh1rows::kh2_atkp`), per-target re-arm. Hits go back to the runtime from KH1's contact. Changes: spell hits now take KH2's knock-back / launch numbers (`scale_record`); a hit reaches the script up to a step later. The scripts themselves and Reflect's barrier meeting are still ours. Steam only. Not checked in game. docs/kh1-magic-native.md §4o.

### 2026-10-26: Stage 639 (testing branch: No KH2-controller borrow for the button fallbacks)
- **No KH2-controller borrow for the button fallbacks (Steam).** The tester (what-is-still-missing, after m5, "(b)": base Sora never on KH2's controller). `shared::control`'s three fallbacks (Square for KH2's dodge roll / guard / Quick Run when `guard_press`, `roll_press` and `quickrun::start` refused, e.g. KH1's deep water `0x1402C16E0`; the air jump when `combomods::aerial_dodge` refused; Square held through KH1's roll when `quickrun::start` refused) leave the press to KH1 as unmodded, as t630 did for the combo. Logged once each. Epic unchanged. Remaining base-Sora borrows listed in the doc. Checked (code); not checked in game. docs/shared-states.md "The last button borrows gone".

### 2026-10-26: Stage 638 (testing branch: KH2's body hits in KH1's own collision pass)
- **KH2's body hits in KH1's own collision pass.** The tester (what-is-still-missing, after m5): "(b), (c) … go ahead". Each KH2 hit window on Sora's body (Vicinity Break 1869, Explosion 911 / 912, Magnet Burst 1900 / 1901 / 1903, also Counterguard's and Finishing Leap's) is a KH1 attack object of Sora's, made with KH1's row maker `0x1402B3270`, tied to his request and to a KH1 attack group of its own (1..3, our slot's hit-window events, `kh2slots::body_windows`). KH1's collision pass (Steam `0x1402B4460` → `0x1402B47A0` → `0x1402B3800`) collides it: `0x1402B3800` is hooked (prologue `40 55 56 57 48 81 EC 80 00 00 00`, Checked (exe)) to put KH2's ellipsoid / column (radius, own half height) and KH2's sweep in as KH1 entries, each tested with KH1's `0x1402B3BE0` (Checked (decomp)). Contact, record, damage and reaction are KH1's own; the ATKP comes through `kh1rows::kh2_atkp`. Per-target re-arm at the ATKP interval (KH2's), KH1's object clock left off. Shapes Checked (files) `P_EX100.mdlx`; KH2 bone 67 → KH1 70. `takeover::kh2_hits` keeps only the body guards and the Magnet pull (still ours). Steam only. Assumption: offsets on bone shapes in KH1's bone frame (none of Sora's has one). Not checked in game. docs/kh1-anim-slots.md §7.z.8.

### 2026-10-26: Stage 637 (testing branch: No fallback: every KH2 pick on a KH1 row of its own)
- **No fallback: every KH2 pick on a KH1 row of its own.** The tester (kh2s-combo-row-selection, after m26): "1. remove the fall back, and make them their own animations if they aren't already". t630's `kh1_own_row` is removed; nothing falls back onto KH1's rows or buttons. Finishing Plus's generic next-finisher rows get a slot of every KH2 finisher of their kind with a row of their own (`kh2slots::prepare`: ground 154 / 155 / 166 / 165, air 183 / 184 / 194 / 197), so when KH2 repeats the finisher playing (a PTYA row whose `sub` is its own id passes the walk, Steam KH2 `0x140402C50`, Checked (decomp) at t374) one of the two rows is always free; KH1's finisher row 8 / 16 no longer plays KH2's motion (`next_finisher`). At Base Combo KH2 with Aerial KH1, KH2's plain hops (rows 0 / 1) play on their own rows (`combo_row`). A pick whose slot didn't build at load is no attack, and a Square / jump move without its row takes the button and does nothing, logged (`own_rows`; `kh1_slots = off` keeps the old plain path). Unchanged: Square with no Square move passing is KH1's roll / guard. Assumption: a finisher on the generic row uses that row's attack data (154's / 183's), its hit numbers still KH2's. 7 more slots baked at load. Not checked in game. docs/kh1-anim-slots.md §7.z.6 (t631, written now) and §7.z.7.

### 2026-10-26: Stage 636 (testing branch: KH2's landing baked into KH1's landing slot)
- **KH2's landing baked into KH1's landing slot.** The tester (same thread, after m3), and m1's "same as kh1 for now" for the landing lock. KH2's landing (motion 5, 26 frames, Checked (files)) on KH1's landing request 7 (slot 11 as template), with the High Jump's 0xC on the same slot (`ALIASES`; both entries `0x200B`, Checked (files)); KH1's player step asks it with action flag 4 on touchdown and queues the idle (Checked (decomp)). The template's events stay at KH1's own frames (style Jump), so KH1's landing lock ("may be left" at 12), dust and sound stay KH1's. No loop point (KH2's lies past its end). `pose::play_for`'s landing stays as the fallback. Not checked in game. docs/kh1-anim-baking.md §11 (and §12, the audit of what is still drawn over KH1's pass).

### 2026-10-26: Stage 635 (testing branch: KH2's idle baked into KH1's idle slots)
- **KH2's idle baked into KH1's idle slots.** The tester (same thread, after m3): "just make sure every animation is baked and on the native system". KH1's idle request 0 (entry `0x4000`, kind 4: the resolver `0x1402A01E0` plays the slot in the field and slot + 1 in battle; Checked (files + decomp)) gets two slots of ours (`kh2slots::CYCLES` now with a slot count): KH2's field idle (motion 0's field variant, 180 frames) on slot 0 and KH2's battle idle (300 frames, Checked (files)) on slot 1, style Misc Animations. Everything that asks KH1's idle (standing, the `0x40000000` action-over request after landings, items, casts, reactions; scripts) now plays KH2's idle in KH1's own pass. The bake takes motions over 254 frames (`bake::reduce` loosens a channel's tolerance until its keys fit a byte; test `long_channels_fit_a_byte_of_keys`); the battle idle comes back within 0.36 units offline on the user's files. `pose::play_for`'s idle stays as the fallback. Not checked in game. docs/kh1-anim-baking.md §10.

### 2026-10-26: Stage 634 (testing branch: Foot IK off)
- **Foot IK off.** The tester (baking-the-rest-of-the-base-animations, after m3): "lets actually have the foot iks off we'll work on this later when we do soras cloth animations". New ini key `foot_ik` (default off) gates both foot placements: KH1's own leg IK and ground lift on the baked slots (t632; `kh1_leg_ik` now needs `foot_ik = on`, so the baked legs are plain FK again) and our own `feet::place` on clips still drawn over KH1's pass (run with `active = false`: it eases out, then leaves the pose alone). Code kept for the cloth work. `feet::place` is the hook's only caller of KH1's floor ray `0x1401910E0` (Checked (code)); KH1's own motions keep KH1's own lift. Not checked in game. docs/kh1-anim-baking.md §9.

### 2026-10-26: Stage 633 (testing branch: The bake's helper channel count)
- **The bake's helper channel count.** A fix to t632 (baking-the-rest-of-the-base-animations, m2: "(a) go ahead"). KH1's pose pass (Steam `0x140391C70`) samples as many helper channels as the motion header's `+0x20` says (Checked (decomp)); `bake::baked` rebuilt the list (travel, the foot helpers' channels dropped and ours added) but left `+0x20` at the template's count, so KH1 sampled too few channels (the foot helpers' last keys never read: legs off the baked ankles) or read past the list. Now written with the list's length; `bake::tests::bakes_a_chain` (failing on testing for this) passes. Found by a second helper's version of the same step. Not checked in game. docs/kh1-anim-baking.md §7.5.

### 2026-10-25: Stage 632 (testing branch: KH1's own leg IK on the baked slots)
- **KH1's own leg IK on the baked slots.** The tester (baking-the-rest-of-the-base-animations, after m2): "(a) go ahead" (option (a): "build KH1's own leg IK onto the baked slots (feet on slopes and stairs), but don't bake idle and landing yet"). Every one of Sora's 118 KH1 motions has the same leg rig: hip 16/42 aimed at foot helper H9/H8, thigh mode-2 two-bone IK, ankle 19/45 at the IK's end turned against the model, foot 20/46 copying the helper. The bake (`kh1_data::bake::baked(.., kh1_legs)`, `kh1_data::legs`) keeps those joint words on our slots and keys the helper (under matrix 0) to the baked ankle and foot, the hip's rotation x to the baked thigh's roll, the shin's rotation x and the ankle's model turn to the baked ones, so KH1's own bone pass (Steam `0x1403900B0`, solver `0x14038FC20`) rebuilds the baked legs on flat ground and lifts them onto slopes and stairs with its own ground lift (Sora's lift list in his model header: thighs 17/43, height 17.434). Our foot IK (`feet.rs`) still runs only on drawn clips (idle, landing, glide). Ini `kh1_leg_ik` (default on). Checked (decomp + exe): the pass, the solver, the matrix conventions. Checked (files): the rig in all 118 motions, the lift list, KH1's replayed pass within 0.19..0.69 units of the pose on ten baked motions (`examples/bake_legs.rs`). Assumption: the instance's lift bits are on during our slots as during KH1's own motions (t348 trace). Per frame: nothing of ours. docs/kh1-anim-baking.md §8.

### 2026-10-25: Stage 631 (testing branch: Every KH2 combo row on a KH1 row of its own)
- **Every KH2 combo row on a KH1 row of its own.** The tester (kh2s-combo-row-selection, after m25): "3. (a) make sure you patch in any features missing in kh1's row so that it can handle it 1:1". `kh1rows::OURS` gains 20 combo rows (`Ours::combo`, one per KH2 motion: 151, 152, 153, 162, 163, 164, 169, 170; 154, 155, 166; 181, 182, 192, 191, 196; 183, 184, 194, 197), each with its own request 0x200 + n and slot, the template row's trail / effect / sound / voice bytes and a copy of its attack data with KH2's ATKP reaction, knock-back, launch and clash rules; no lunge (btltbl rows at +0xA9F0, Checked (files)). KH1's picker returns the row itself (`combomods::combo_row`); KH1's Attack step runs the picker's return in states 0 / 2 (Steam `0x1402A7510`, Checked (decomp)). KH1's plain requests 0xC8..0xCE are no longer re-pointed per press (fewer bakes at load). What KH1 reads from `+0x164`: the Attack step's Fire out of a 1st / 3rd swing sees the template request for that call (`combat::command` wrapper); event opcode 322 (Dive to the Heart's tutorial) gets the template request (`probe::event_op`), both Checked (decomp). Assumption: `0x14039A4A0`'s effect pick (200 / 0xD2..D6) isn't Sora's. Change in play: KH2's plain swings and finishers land with KH2's knock-back. Not checked in game. docs/kh1-anim-slots.md §7.z.6.

### 2026-10-25: Stage 630 (testing branch: No KH2-controller borrows in the combo)
- **No more KH2-controller borrows in the combo.** The tester (kh2s-combo-row-selection, after m25): "2. (b)". The borrow path (`combomods::want` / `take_want` / `lent` / `before_step`, `takeover::shared_frame`'s ability case) is removed. An Attack whose KH2 pick no KH1 row can play (slot not built, no free row for a further Finishing Plus finisher) runs KH1's row its picker chose (`kh1_own_row`; at the KH2 setting, picked with KH2's length); a finisher there counts on Finishing Plus's chain. Square moves, Retaliating Slash and Aerial Recovery with no KH1 row leave KH1 its own Square / jump. `Run::Kh2` is now `Run::Nothing`. Assumption: the picker row at the KH2 setting is the one picked with KH2's length. Not checked in game. docs/kh1-anim-slots.md §7.z.5.

### 2026-10-25: Stage 629 (testing branch: Zantetsuken follows KH1 Ability Chance)
- **Zantetsuken follows KH1 Ability Chance.** The tester (kh2s-combo-row-selection, after m25): "no it should change on the toggle we have like the other moves, if you have ability chance off then it should be 100 percent like the other moves, if not then 30% chance is back". `combomods`' Zantetsuken step takes `Kh1Rule::Chance` like Stun Impact and Gravity Break (t589's always-roll rule is gone). With the Tweak on, it is KH1's third 30% roll on the same random number (Steam `0x1402A6EE0`, Checked (decomp) at t580/t589); with it off, it passes whenever its single-target group's checks pass. The Tweaks help text names it. Not checked in game. docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 628 (testing branch: No hair switches)
- **The hair switches out again.** The tester (missing-sleeves-animation, after m21): "2.(b)", i.e. take the `hair_physics` / `kh2_hair` switches out again without changing anything else. Both ini lines, their settings fields, the old `physics = on / off` fallback and its migration in `save_physics` are gone; `pose` always runs KH2's own animation of the hair, hood, sleeves and keychain, as the switches' defaults (on / on) did. Nothing else changed: `jiggle.rs` still swings the chains KH2 doesn't pose this frame (the dream weapons' and wooden sword's keychain), as it does with the switches on. No addresses. Checked (code: every reader of the two fields); not checked in game. docs/kh1-secondary-motion.md "Switches".

### 2026-10-25: Stage 627 (testing branch: Jump and High Jump baked into KH1's takeoff)
- **Jump and High Jump baked into KH1's takeoff.** The same thread (plan step 4). `kh2slots::JUMPS`: KH2's jump (3, 26 frames) on KH1's takeoff 0x4 (template slot 8) and its High Jump (201, 64 frames) on 0x9 (template 12), style Jump, held at the end until the switch into the fall. KH2's jump start (`0x140422630` / `0x1403B59B0`, Checked (decomp)) sets the fall motion to 4 for both, so 201 is cut at apex − 5 as the plain jump is (the exe's rule, not the OG rig's "plays to its end"; 201 has no range id 49, Checked (files)). KH1's apex slots 9 / 13 are never asked at Jump KH2. KH1's takeoff sounds and foot effects stay at KH1's own frames (the takeoff is frame 4 in both). Assumption: KH1's takeoff bit 16 holds at the end. Not checked in game. docs/kh1-anim-baking.md §7.3.

### 2026-10-25: Stage 626 (testing branch: The fall baked, and KH2's switch into it)
- **The fall baked, and KH2's switch into it.** The same thread (plan step 3, after m1: "go ahead"). `kh2slots::FALL`: KH2's fall (motion 4) on KH1's fall requests 6 and 0xB (`ALIASES`; both slot 10, Checked (files)), style Jump. KH1 loops it from KH2's loop point 12 of 32 through KH1's own loop range (`+0x160` bit 0x40, `+0x288` / `+0x28C`; `0x1402A0350`, `0x14029F0A0`, setter `0x14029FA10`, Checked (decomp)). At Jump KH2, KH1's apex step (Steam `0x1402AF630`: its apex base+5 with blend 6.0 at `+0x3B0`+6 ≥ `+0x3B8`, then its fall queued, Checked (decomp, exe)) is wrapped (prologue `40 53 48 83 EC 20`, Steam only) to run KH2's rule (KH2 `0x1403CB680` → `0x1403CB610`: the fall motion `+0x118` = 4 with blend 8.0 at apex − 5, at once on an early release; Checked (decomp)): KH1's fall request with blend 8, no apex motion. Early releases ask it from `shared::physics`. `pose::follow_baked` keeps the baked frame's KH2 pose, so a drawn clip after a baked one blends (the drawn landing out of the fall; also baked walk / run into the drawn idle, which snapped). Assumption: KH2's arc clock and blend are in 60 Hz frames. Landing and idle still drawn over (answer 1 open). docs/kh1-anim-baking.md §7.2.

### 2026-10-25: Stage 625 (testing branch: Item use baked into KH1's item step)
- **Item use baked into KH1's item step.** The tester (baking-the-rest-of-the-base-animations, after m1): "3. (b) kh2 voice plays if the toggle for kh2 voices is on … go ahead". `kh2slots::ITEMS`: KH2's item use (motion 9) on KH1's item requests 0x3E / 0x3F (templates 58 / 59), Misc Animations KH2. KH1's command step (Steam `0x1402A3DB0`, type 3: blend 8, the idle queued with `0x40000000`, the item's task `0x1402C2760` not tied to the motion, Checked (decomp)) plays it in its own item step. KH1's "may be left" is KH2's free state (frame 36 of 50, Checked (files)); the end is KH1's queued idle. KH1's item sound is left out; KH2's sound 305 at 10 always plays (`kh2sound::allowed`), its voice 3 only with KH2 battle quotes. `takeover::item_use_now` ends with KH1's item slot instead of the 51-frame clock. Not checked in game: the heal's timing against frame 36. docs/kh1-anim-baking.md §7.1.

### 2026-10-25: Stage 624 (testing branch: Finishing Plus after a finisher cast)
- **Finishing Plus after a finisher cast.** The tester (kh2-magic-and-reflect-native, after m10): "2. (a) go ahead". Steam KH2 `0x1403D6CA0` takes an Attack, and from the input step `0x1403D6FC0` a spell (command kind 2), in the finisher window (range id 50, `+0x194` bit 0x20) while the finisher count `+0xD75` is under the maximum `[+0x5C0] + 0x202`; both counts go up by one (Checked (decomp)). The ground finisher casts' range 50: Fire 57 54..84, Blizzard 60 46..76, Thunder 63 / 68 42..78, Reflect 72 54..84; air casts none (Checked (files), `P_EX100.mset`). Base Sora's native casts and Reflect are now noted as combo steps (`combomods::cast_step`: `LAST`, `FINISHERS`; `formmagic::note_step`'s `CHAINED`). A ground finisher cast with a finisher left (`combomods::finisher_left`: under 1 + Finishing Plus) keeps KH1's count `+0x41` at its step instead of 0; `kh2slots::finisher_window` opens KH1's combo window there only while one is left; `kh2slots::cast_finisher_open` lets `combat::command` take an Attack or a spell (items stay range 11). The next finisher is picked as after a finisher swing; a spell there is the next finisher cast. Assumptions: the maximum is 1 + Finishing Plus; KH1 restarts a cast request already playing. Drive Forms unchanged (answer 1). Not checked in game. docs/kh1-magic-native.md §4c.2.

### 2026-10-25: Stage 623 (testing branch: KH2's table walk without a controller copy)
- **KH2's table walk planned without a controller copy.** The tester (combo row selection m24), step 2 of the native plan. In `character-core`, `Character::plan_press` is KH2's press step as a pure function of a `PressState` (counts, hit, carried combo, air, facing, forced row, host motion, sideways speed): count / finisher step, lock turn, selector walk `combo::select` (Steam KH2 `0x140402D20`), forced-row and Square rules, hop (`0x1403D78D0`) or lunge. `try_attack` is `plan_press` applied; `would_take_launch` / `would_take_row` (and so `ability_row_id` / `normal_finisher`) build the press state from KH1's data and run the plan instead of cloning the controller and pressing Attack on the clone. Checked: unit test `host_trials_walk_the_table_as_the_copy_did`, 34,560 cases both ways, same row and hop. Assumption: `try_attack` unchanged by the refactor (read and the 67 sim tests, not a frozen copy). The PTYA table and Sora's numbers are still read from the KH2 Character object. Expected: no change in play. docs/kh1-anim-slots.md §7.z.4.

### 2026-10-25: Stage 622 (testing branch: KH2's pick inside KH1's picker)
- **KH2's whole pick made in KH1's picker at the KH2 setting.** The tester (combo row selection m24): "(a)" / "go ahead", step 1 of the native plan. KH1's Attack step `0x1402A7510` asks its ground picker `0x1402A6EE0` (state 0) or air picker `0x1402A6C70` (states 2 / 8) and runs the returned row with the row runner `0x1402A68F0`; each has only that caller and state 1 runs no row, so the picker always runs on the same frame just before the row it picks (Checked (decomp), Steam). `kh1picker::at_kh2` now makes KH2's pick once (`combomods::kh2_press`, returning a `Press`: row, length, Negative Combo, hop, chained), runs KH1's picker with KH2's length and its ability finisher bits held back, then the t612 walk (`kh2_finisher_walk`, now `Option<i32>`); where KH2's pick stands, `kh2_pick_in_picker` returns the KH1 row that plays it (`kh2_row`, moved from the row runner; dash distance at `dir+4` cleared when it replaces rows 0 / 1) and keeps the pick for the row runner (`KH2_PICK`: same frame, row, air). `combomods::attack` takes the kept pick instead of asking KH2's selector again and no longer swaps the row afterwards. One KH2 trial per press instead of two. Expected: no change in play; not checked in game. docs/kh1-anim-slots.md §7.z.3.

### 2026-10-25: Stage 621 (testing branch: Aerial Finish and Magnet Burst follow Physics again)
- **Aerial Finish and Magnet Burst follow the Physics row again (t617 undone).** The tester (air-physics-for-kh2-magic-aerial-finish-and-magnet-burst m3): "I misunderstood, abilities should follow the main physics config." t617's code is reverted: `shared::physics::{AIR_PHYSICS_MOVES, AIR_MOVE_AT, air_physics_hop, air_physics_move}` and the `combomods::air_hop` note are gone, `kh2_hop` / `air_step` take `kh2()` again for these two moves (air casts still `cast_kh2()`), and the Air Physics (`0x7F00_0190`) and Physics (`0x7F00_011A`) help texts are back to their pre-t617 wording. The research stays in docs/kh1-magic-native.md §4i, rewritten: KH2 plays both moves inside ACTION_JUMP like its casts (row hop `0x1403D78D0`, step `0x1403CB360`, Checked (decomp)); which Config row covers them is our choice. Checked: a reverse of t617's code files; not in game.

### 2026-10-25: Stage 620 (testing branch: Sleeves: back to t375)
- **Missing sleeves animation: back to t375.** The tester (missing-sleeves-animation, after m20): "I want you to revert changes made in this thread to "> t375 ..." and bake the animations for me. we'll figure out a way later, keep the debug menu frame and frame count changes". Reverted, by intent per commit (other threads' edits in the same files kept): t385–t388, t405–t407, t411, t412, t453, t490, t491, t499, t500, t519, t552, t569, t592, t595, t596, t613, t615, t616. Back from t375: KH2's sleeve flaps on KH1's four sleeve chains plus hair, hood and keychain from KH2's keys, live and baked; the `hair_physics` / `kh2_hair` switches and `jiggle.rs` for chains KH2 doesn't pose; the coat and shorts on t375's rules; the thigh roll by the knee's direction. Kept: t389 (KH2's walk / run baked into KH1's slots; later work builds on `kh2slots::CYCLES`), t593's Frame Advance panel (its cloth line removed), t594 (Frames row), t614 (Animation label). Removed files: kh1-data `cloth.rs`, `clothsim.rs`, their local tests and examples, kh1-hook `win/clothlive.rs`. The bake (`pose::Baker` → `kh2slots::bake_one` → `kh1_data::bake`) is built at load from the player's own files, so every baked slot carries t375's motion with nothing stored in the repo. Checked (git history); not checked in game. docs/kh1-secondary-motion.md "Back to t375".

### 2026-10-25: Stage 619 (testing branch: Medium Hits on the ground and in normal air)
- **Medium Hits only on the ground and in normal air.** The tester (soras-hit-reactions-native m3, answer 3): "(b)". KH1's hit pass (Steam `0x14029BBD0`) picks its reaction function by Sora's movement state `+0x70`: the ground row for 0, 1, 0xC, 0x1C, 0x20, 0x26, 0x18, 0x19; own rows for flight 8, swimming 0x11, 0x1D, 0x1E / 0x1F and 0x27; the plain air row otherwise (Checked (decomp)). `hitreact::medium_hits` (kind 4 → 3 before the pass) now runs only in the ground and plain-air states; flight, swimming, Atlantica's water 0x18 / 0x19, the carpet 0x1D, the Jungle Slider 0x1E / 0x1F and 0x27 keep KH1's kind 4 (`medium_state`). Answers 1 and 2 needed no code (Battle Quotes already decides the hurt line; the flight flinch's animation already follows Hit Reactions through `kh2slots::redirect`). Assumption: 0x18 / 0x19 counted as swimming from earlier stage notes; 0xC / 0x1C / 0x20 / 0x26 unidentified, kept as KH1's ground. docs/kh1-hit-reactions-native.md §12, §16.

### 2026-10-25: Stage 618 (testing branch: Glide: KH2's blend clock)
- **Glide: KH2's blend clock on the glide's motions.** The tester (kh2-glide-in-kh1s-glide-state, after m6): "go ahead with any changes you need to make it 1:1". Both games hold a new motion at its start through a blend (KH2's motion update `0x1403C6740`: time `+0x4C` held while the blend clock `+0x44` runs to `+0x48`; KH1 Steam `0x14029F0A0`: `+0x174` / `+0x170` / `+0x16C`), so the bank restart is KH2's own (Checked (decomp), and the tester's log). The differences: KH2 starts a blend's clock at 2 (`0x1403C8A40`) and asks the glide's motions from the action update after that frame's motion update (vtable `+0x28`, `0x1403B2DA0` → `0x1403B4780` → `0x140423A70`, run by `0x1403BFD30`; Checked (decomp, exe vtable / RTTI)), while KH1's apply `0x1402A0350` starts `+0x174` at one frame step; and KH2 turns a time stepping from 0 to 1 into 2 (`0x1403C6740`). `glide::kh2_clock`, in the glide pre-step, for 205..208 only: on the first frame after KH1's apply `+0x174` = 2; on the first frame after a blend ends at time 0 (60 fps) `+0x16C` = 1, which KH1's update makes 2. Left: the request frame still draws KH1's 1/16. Assumption: KH2's motion step is 1 per 60 Hz frame. No new hooks. Logged (first 12). docs/kh1-glide-native.md §8.10.

### 2026-10-25: Stage 617 (testing branch: Aerial Finish and Magnet Burst follow Air Physics)
- **Aerial Finish and Magnet Burst follow the Air Physics row.** The tester (air-physics-for-kh2-magic-aerial-finish-and-magnet-burst m1, answer 1): "(b)". KH2 plays both moves (motions 194 / 197) inside its jump state like its air casts: the combo row's hop `0x1403D78D0`, then ACTION_JUMP's step `0x1403CB360` with its hold, running clock and fall cap (Checked (decomp)). `combomods::air_hop` notes the pick (`shared::physics::air_physics_hop`), and `kh2_hop` / `air_step` take `cast_kh2()` instead of `kh2()` for these two moves (`air_physics_move`: the motion playing, or its hop asked in the last 2 frames); the off-arc rule casts get at Air Physics KH1 under Physics KH2 covers them too. At KH2: KH2's hop on KH2's gravity, the hover with the clock running, then the drop at 16 a frame; at KH1: KH1's gravity, the hover freezing KH1's clock, KH1's fall. Help texts of Air Physics (`0x7F00_0190`) and Physics (`0x7F00_011A`) updated. Drive Forms unchanged (answer 2). Assumption: the move is told by KH2's motion id on our slot plus a 2-frame window for its hop. Not checked in game. docs/kh1-magic-native.md §4i.

### 2026-10-25: Stage 616 (testing branch: Cloth: the coat calmer)
- **Cloth physics: the coat much calmer.** The tester: "the bottom part of soras coat ... moves way too much, kh1 soras coat is way less pronounced then that". KH1's own field run moves each hem tip at most 0.3-4.6 units from its mean place against the chest (rms 0.2-2.7) (Checked (files)); t592's coat physics, driven by KH1's run in an offline copy, moved the tips up to 5.7 on a stop, 5.5 on a start and 4.6 in a 90° turn. New coat settings: stiffness 0.02→0.06, damping 0.30→0.40, weight 0.25→0.10, cap 40°→20° (1.5 / 1.5 / 0.8; the stride and bob response ¼ to ⅖ of t592's). `Settings::TUNED`; settings id 0 / Reset mean these numbers. The tester's own tuned values weren't in the trace (Cloth category off), so not applied. Assumption: KH2's own run's excitation not evaluated. docs/kh1-secondary-motion.md "The coat calmer".

### 2026-10-25: Stage 615 (testing branch: Cloth: links' middles clear the thighs)
- **Cloth physics: each link's middle kept out of the thighs.** The tester: "sora at the peak of his run around frame 11 is clipping through his shorts". t569's clearance rules were tested only at each link's tip, so with a knee up a link could pass through the thigh between two clear points. Each link's middle is now held to the same rules (each point with its own bind-distance cap), the link swung about its start to clear it. Assumption: which part clipped at frame 11 (coat, belt or shorts link over the rising thigh) isn't known; not checked against KH2's run frame by frame. docs/kh1-secondary-motion.md.

### 2026-10-25: Stage 614 (testing branch: Frame Advance panel: Animation)
- **Frame Advance panel: the frame line reads Animation.** The tester: "I assume field frame run is the animation count, change it to animation so its understandable". The panel's line "Frame: field run 12 / 40   Loop 3" now reads "Animation: field run, frame 12 / 40, loop 3" (`fapanel::lines`). docs/kh1-frame-advance.md.

### 2026-10-25: Stage 613 (testing branch: Coat hangs from the chest)
- **Cloth physics: the coat hangs from the chest.** The tester (missing-sleeves-animation, after m19): "the part of the coat next to soras necklace doesn't move with the rest of the coat so the mesh stretches". t592 put each coat root (72, 248..276) back on the spine base 71 at KH1's run relation, but the retarget turns 71 only 67% from KH2's pelvis toward KH2's chest while the chest 78 (the jacket by the necklace) turns fully, so the coat's top ring sheared against the jacket in KH2's run. In KH1's field run 71 and 78 stay rigid to each other and no coat root is keyed (only r+1 / r+2) (Checked (files)). The coat's carrier is now the chest 78, each root held on it at its bind relation (as the retarget's `ATTACHED` drew it before t592); the joints below swing as before. The trace's Cloth body bones gain 78. Assumption: the jacket band by the necklace is skinned to the chest (skin weights aren't read). `kh1-data/src/clothsim.rs` (`CHEST`, `Chain::held`), `clothlive::BODY`, docs/kh1-secondary-motion.md.

### 2026-10-25: Stage 612 (testing branch: KH1 finishers at the KH2 setting)
- **KH1's ability finishers at the KH2 setting, in KH1's picker.** The tester (m23): "I noticed I can't do any kh1 finisher on kh2 combat settings", "patch features into kh1's engine". At Base / Aerial Base Combo KH2 the row runner replaced KH1's ability finisher rows with its plain row whenever KH2's pick was plain (`kh2_row`, t390), so KH1's moves never came. Our hooks on KH1's pickers (`0x1402A6EE0` / `0x1402A6C70`, `kh1picker::at_kh2`) now also run at the KH2 style: KH2's pick at this press first (`combomods::kh2_press`); KH1's picker runs with its ability finisher bits held back (`HELD_GROUND`) and KH2's combo length on the length byte it reads (stats `+0xD4` / `+0xD5`, test `len <= count`, Checked (decomp)); when KH2's pick is a finisher the t580 walk runs (`kh2_finisher_walk`: KH2's table order, KH1's twins in their KH2 places, KH1's moves on their KH1 rules via `Kh1Rules`, no dash exception), and a KH1 move that passes is the row KH1's picker returns, run whole (`take_kh1_take`). Finishing Plus: `kh1_finisher_window` also opens on KH1's finishers at the KH2 setting by KH2's count, so KH1 → KH2 and KH2 → KH1 finishers chain. Cost: one more KH2 trial per Attack press at this setting. Assumption: KH1's moves keep their KH1 rules at the KH2 setting. docs/kh1-anim-slots.md §7.z.2.

### 2026-10-25: Stage 611 (testing branch: Group rows count every enemy at KH2)
- **KH2 setting: group rows count every enemy while locked on.** The tester (kh2s-combo-row-selection m23, answer 1): "(a)". KH2's selector `0x140402D20` runs its kind-2 pass `0x1404029F0` on every target its iterator `0x1403BDAC0` returns; the pass only sets the row's bit in the selector's mark array (`+0x60`) and keeps no target, and the hop and lunge (`0x1403D78D0`) are measured to the character's own primary target `+0xDC8` (Checked (decomp)). `Character::try_attack` (the KH2-setting trial) no longer keeps only the locked foe (stage 207's filter is gone), and `combo::select` aims a kind-2 row at the primary target instead of the one that passed it, so the "lunges at the foe beside him" problem stays fixed the way KH2 fixes it; unlocked play changes the same way. Test `group_row_passes_on_any_target_and_aims_at_the_primary`. docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 610 (testing branch: KH2's teamwork block)
- **kh2-magic: KH2's teamwork block shared by every script (Reflect's chain).** The tester (m9), and a correction of t572's assumption. Steam KH2 `0x1403F6DF0` (`trap_teamwork_alloc`) keeps one block per key (table `0x142AE75D0`, count `0x142AE7650`): size + 1 zeroed bytes the first time, then byte 0 set to 1 and the same block (Checked (decomp)). Reflect's script (key 0x10002) ends the previous barrier element stored at +4 at release and stores its own, adds its count to +8 when its barrier ends, and only the newest cast clears the block and fires the counter, tiered by the chain's total (Checked (files)). kh2-magic gave each cast its own block; `Magic::teamwork` now holds the shared bytes, synced into and out of each cast's memory around its run; `reflect.rs` no longer resets the chain note and counts on a chained cast. Test `reflect_chain_shares_the_teamwork_block` (needs extracted files; not run here). docs/kh1-reflect-spell.md, docs/magic-model.md.

### 2026-10-25: Stage 609 (testing branch: Effect lengths in 30 Hz ticks)
- **kh2-magic: effect elements live their length in 30 Hz ticks.** The tester (m9): "reflect is still sending its shots on the first hit". The traces (f42073 / f42322 / f42330) show every barrier attack standing exactly 27 frames, the counter's element and the 574 hits the frame after (Checked (trace)). Reflect's script keeps its barrier while element 0 lives (`trap_effect_is_alive` loop), then counters (Checked (files), `REFLECT_3.mag`). Particle-data lengths are effect ticks, which KH2 runs at 30 Hz (`0x1402C1D80`, Checked (decomp)), but `Effect::alive` compared game frames with ticks, so every element died at half its length while KH1's effect engine still drew it. `effect::FRAMES_PER_TICK` = 2; ages are compared in ticks. Test `effects_live_their_length_in_30_hz_ticks`. Side effect (1:1): every kh2-magic script that waits on an element now gets KH2's length (Blizzard's shard flies twice as long as before). docs/kh1-reflect-spell.md.

### 2026-10-25: Stage 608 (testing branch: Attack out of a cast only in its combo window)
- **Attack out of a KH2 cast only in its combo window.** The tester (m9, answer 2): "(a)". Steam KH2 `0x1403D6CA0` takes an Attack only in the combo window (range 11, `+0x18C` bit 0x400), the finisher window (range 50, `+0x194` bit 0x20, Finishing Plus left) or a free state; the reaction-command window (range 14) is only for reaction-command rows (`0x1403BA070`). Checked (decomp). In `P_EX100.mset` only Blizzard 59 has range 11 (from frame 36); the finisher casts 57 / 60 / 63 / 68 / 72 have range 50; no cast has a free state. Checked (files). `kh2slots::cast_follow`: KH1's combo window on our casts now covers range 11 plus range 35 (the magic window), not range 14 to the end; new `kh2slots::cast_attack_window`; `combat::command` refuses Attack (silently) and items (KH1's "can't") outside range 11; KH2's controller (Drive Forms / borrows) takes an Attack in a cast only in range 11. Test `attack_out_of_a_cast_only_in_its_combo_window`; the `magic_sora` / `spell_mid_combo_is_a_step_of_it` test clips now carry a combo window. Not built: Finishing Plus in a finisher cast's range 50. docs/kh1-magic-native.md §4c.1, kh1-anim-slots.md.

### 2026-10-25: Stage 607 (testing branch: KH1 spells: no jump or Square cut)
- **KH1 spells: no jump or Square cut.** The tester (kh2-magic-and-reflect-native m9, answer 1): "(a)". `takeover`'s hand-off path (a KH1 spell started out of one of KH2's moves, and a Drive Form's KH1 spell) let a jump or Square end the cast once the spell was out (s168's "other cut"); that cut is removed, so jump and Square wait until KH1 lets him act. Attack or another spell still cut the cast as before (`attack_cut` / `magic_cut`). KH2 takes a jump or the action button only in a free state (Steam KH2 `0x1403D6FC0` / `0x1403BFB30`), which no cast has. Checked (decomp). docs/kh1-anim-slots.md.

### 2026-10-25: Stage 606 (testing branch: Tweaks: Enemy Weight)
- **Config: Tweaks > Enemy Weight.** The tester (kh2-knockback-on-enemies m1, answer 1): "(b) but make it a toggle in the config menu". Combat > Tweaks gets a second row, Enemy Weight (Off | On, text ids `0x7F00_0193` / `0x7F00_0194`), switching `knockback::weight` live and saved as `enemy_weight` (default on). On: KH2's knock-back × KH1's per-enemy weight (t602-t605); Off: KH2's numbers for every enemy. docs/kh1-config-menu.md §6.y.

### 2026-10-25: Stage 605 (testing branch: KH2's air flinch)
- **KH2's air flinch in KH1's enemy air flinch.** The tester (kh2-knockback-on-enemies m1, the plan's step 3). A new Steam hook on `0x1402B8B90 (actor, record, float front)` (third argument in XMM2), prologue `48 89 5C 24 18 48 89 74 24 20 57 48 83 EC 60` (Checked (exe)). After KH1's runs (its motion 0x4E / 0x4F, hop 20, drift 4 × weight), for a KH2 record KH1's arc fields take KH2's 50 hop and the flight is KH2's: the hop on KH2's jump curve with a constant drift of push distance × 0.048 (× weight with Enemy Weight on) along the record's direction (KH2 `0x1403D4800`: 50.0 and sstm `+0x38`, Checked (decomp)). Assumption: the constant drift follows `character_core::npc`'s model. Not done: KH2's turn of the flinched body (`0x1403D3840`). docs/kh1-launch.md §4.

### 2026-10-25: Stage 604 (testing branch: KH2's push in KH1's flinch push)
- **KH2's push in KH1's enemy flinch push.** The tester (kh2-knockback-on-enemies m1, the plan's step 3). A new Steam hook on `0x1402B7E60 (actor, record)`, prologue `48 89 5C 24 18 57 48 83 EC 40` (Checked (exe)); KH1's runs first, then for a record a KH2 hit made (`knockback::note_record` from `combat::scale_record`, fresh 250 ms) KH1's push speed `+0x3FC` becomes KH2's distance × 0.1 (KH2 `0x1403D4960`): knock-back 1, or 200 when it is 32767, halved for 0xC / 0xD; a plain flinch (group 2) gets none; × stats `+0xC4` with Enemy Weight on. Before each enemy physics step (`takeover::physics`, Steam `0x14029A3C0`) KH2's ×0.9 a frame decay replaces KH1's ×0.8 in KH1's own push step `0x1402C7070` (KH2 `0x1403D4570`). Checked (decomp) both games. Left to KH1: enemies of weight 0, KH2 blows on enemies KH1 can't launch, KH1's own pushes, spells' hits (`spell_strike`). docs/kh1-launch.md §4.

### 2026-10-25: Stage 603 (testing branch: KH2's blow flight on KH1's air state)
- **KH2's blow flight on KH1's enemy air state.** The tester (kh2-knockback-on-enemies m1, the native plan's step 2). The `shared::physics::air_step` wrapper (KH1's state-2 step, Steam `0x1402AF6A0`, through the state table) now calls `knockback::air_step` for enemies a KH2 blow launched and replaces the step's displacement with KH2's `Blow`: one 30 Hz tick every second frame, sideways ×0.8 floored at a tenth of the start, rise ×0.8, gravity 20/49, fall capped at 16, with `+0x3C0` / `+0x3C4` kept in step. KH1's mover, collision, landing, lying and get-up stay KH1's. The flight ends when the enemy leaves state 2, when KH1 writes another arc, or after 600 frames. Checked (decomp): KH2's velocity provider `0x140410330`. Assumption: the 30 Hz tick (docs/combat-model.md), not re-checked against a KH2 trace. docs/kh1-launch.md §4.

### 2026-10-25: Stage 602 (testing branch: KH2 blows through KH1's launch)
- **Every KH2 blow launches through KH1's launch.** The tester (kh2-knockback-on-enemies m1): "(b) but make it a toggle in the config menu / (a) / (a) / go ahead". KH2's reaction routine (Steam KH2 `0x1403D2FE0`, groups by `0x1403CE980`) gives a blow its starting speeds in `0x1403CEA00`: up = ATKP knock-back 1, sideways = knock-back 2, the defaults 24 / 40 only when both are 0 (Checked (decomp)). Every group-4 hit now takes KH1's finisher row and a kind-5 record (`kh1rows::throws` is the group alone); after KH1's launch (Steam `0x1402B8530`, existing hook) KH2's blow is flown to its apex for `+0x3B8` / `+0x3BC` and `+0x3C0` gets KH2's sideways speed. Gone (answer 3 (a)): the "holding blow" rule (`Strikes::hold`, `HOLD_*`, the zeroed record direction) and Explosion's override `FARTHER` (912 is KH2's 15 up / 30 sideways). A KH2 flinch on one of our rows whose data launches gets KH1's flinch (kind 3). Enemy Weight (answer 1 (b), `knockback.rs`, ini `enemy_weight`, default on): launches × KH1's launch speed for the enemy (20 + 15 × stats `+0xC4`, btltbl `*0x142D22D30` `+0x14` / `+0x16`, Checked (decomp)) over 35. Assumption: stats `+0xC4` = 1 is KH1's full weight; each enemy's value is logged once. Distances unscaled (answer 2 (a)). docs/kh1-launch.md §4.

### 2026-10-25: Stage 601 (testing branch: R2 Camera Hold)
- **The camera hold option on R2.** The tester (kh2-camera-fixes, after m15): "R2 should be the button for locking the camera". t498's L2 Camera Hold is now **R2 Camera Hold**: the Config > Camera row is renamed (`Row::R2Hold`, text `0x7F00_0121` / `0x7F00_0122`), ini key `r2_camera_hold` (an ini with the old `l2_camera_hold` keeps its choice), `camera::R2_HOLD`. R2 is read by the same `r2_held` as the t413 camera hold (Steam raw pad word `0x14298C9B4`, bit `0x200`, so the keyboard's R2 counts). With it on and R2 held, KH2's follow and lock-on cameras get no right-stick turn or zoom and no mouse motion, and R3 doesn't reset; R2 still stops the swing-behind (Auto) as in t413. L2 Stick Menu stays on L2. Defaults stay Off and the two options stay separate (the tester's (b) / (b)); L2 + mouse is not done ((c), later). With Frame Advance on, R2 steps frames instead. Assumption: the button is the tester's choice (KH2's run-time remap table can't be read). docs/kh2-pad.md.

### 2026-10-25: Stage 600 (testing branch: Config: KH2 Damage row)
- **KH2 Damage row.** The tester (m3, answer 3): "(a)". A Config > Combat row "KH2 Damage: Off / On" for ini `kh2_damage` (default On, live, `kh2damage::kh2_damage` / `set_kh2_damage`). On: KH2's power on KH1's curve, as before. Off: KH1's damage exactly; the curve isn't swapped and `combat::scale_record` no longer rewrites the damage after the hit (the old after-the-fact rewrite now runs only where the hook can't be made, Epic). `damage_scale` / `magic_scale` act only with it On; reactions, launches and criticals don't depend on it. The Combat list is 12 rows now, drawn smaller by `list_scale` (about 0.74). Checked: code paths. docs/kh1-kh2-attack-data.md §9.

### 2026-10-25: Stage 599 (testing branch: Config: Party Criticals)
- **Party Criticals.** The tester (m3, answer 2): "in the config menu make a separate option for party member criticals". A new Config > Combat row "Party Criticals: Off / On" (ini `party_criticals`, default Off, live) decides instead of Criticals for an attack whose owner (`+0x34`) is an actor on Sora's side (`+0x374` bits 5..6 equal to his) and isn't Sora: Donald, Goofy and the guests (`kh2damage::party_member`). The side bits are the ones KH1's damage function itself tests (Checked (decomp)). Assumption: every actor on that side except Sora counts (a summon's own attack would follow this row too); an attack whose owner is gone follows Criticals. docs/kh1-kh2-attack-data.md §8.

### 2026-10-25: Stage 598 (testing branch: Criticals Off for every attack)
- **Criticals Off applies to every attack.** The tester (kh2-attack-data m3, answer 2): "lets do (b)". KH1 rolls its critical in its damage function `0x1402BFA80` for every attack, whoever owns it: a random value below the attack's `+0x94` adds `+0x98` to the power and sets record bit 0x20; only the journal tally (`0x140291FF0`) is the player's, and object targets never roll. Chance and bonus are set when the attack object is made (`0x1402B3270`: weapon factor on `+0x94`, bonus on `+0x98`). Checked (decomp). `kh2damage::damage` now zeroes `+0x94` for the call on any attack while Criticals is Off (Sora's, the party's, enemies') and restores it after; On is KH1's roll exactly. Help text and ini comment updated. Not checked: which enemies' attack data can crit. docs/kh1-kh2-attack-data.md §8.

### 2026-10-25: Stage 597 (testing branch: Aerial Dodge: no release at Physics KH1)
- **Aerial Dodge: no early release at Physics KH1.** The tester (aerial-dodge-native m3): "(b)" (no release at Physics KH1: the dodge always rises fully), "go ahead". On the frame a dodge starts at Physics KH1, `shared::physics::kh1_held_jump` now clears Sora's `+0` flag `0x40000` (KH1's early-release cut still open) instead of setting t545's `0x60000`. KH1's button handler `0x1402A7B30` (Steam, from the control step `0x1402A6220`) runs its variable-jump cut (`0x1402AF8F0 (actor, 5.0)` from air time 6) only while that bit is on, so the dodge always rises fully, held or let go; the fall still swaps at the apex (`combomods::dodge_apex`). No other KH1 code reads that bit on the actor (the other `0xE0000` tests are on attack objects). Checked (decomp). Physics KH2 unchanged (KH2's own release, `0x1403CB680`). docs/kh1-aerial-dodge-native.md §8.

### 2026-10-25: Stage 596 (testing branch: Debug menu: Cloth Physics page)
- **Debug menu: Cloth Physics page.** The tester (m19 6 (a and b)): the field-run physics on / off and each part's stiffness, damping, weight and swing cap live, plus Reset to tuned; the settings id in the page title and every change logged ("cloth physics settings id N ..."); session only. docs/kh1-secondary-motion.md.

### 2026-10-25: Stage 595 (testing branch: Trace: Cloth category)
- **Trace: Cloth category.** The tester (m19 3), off by default (ini `trace_cloth = off`, its own group): the animation drawn as the panel shows it, the physics state (on, baked / live, steps, restarts, settings id, max swing, tips pushed out), Sora's place / yaw / speed / turn rate per 60 Hz frame, every physics bone's turn against its parent and place against the hips, and the collision bones (3, 4, 17, 18, 43, 44, 71). docs/kh1-secondary-motion.md "the trace's Cloth columns".

### 2026-10-25: Stage 594 (testing branch: Trace Recorder: Frames)
- **Trace Recorder: Frames.** The tester (m19 2): with Game Frames on, the Seconds row reads "Frames" and shows seconds × 60 (it still steps by a second; the ini keeps seconds). docs/kh1-frame-advance.md.

### 2026-10-25: Stage 593 (testing branch: Frame Advance info panel)
- **Frame Advance info panel.** The tester (m19 1 (a), 2 (a)): a top-right box while Frame Advance is on, apart from the debug menu (`fapanel.rs`, drawn from `debugmenu::draw_end`): KH2 motion, battle / field, KH1 slot (and ours), drawn baked / live / by KH2's controller / KH1's own, the frame within the animation ("field run 12 / 40"), loops, the trace frame, cloth settings id and steps. Assumption: KH1 wraps a looping motion's time at its length. docs/kh1-frame-advance.md "The info panel".

### 2026-10-25: Stage 592 (testing branch: Cloth physics live on KH2's field run)
- **Cloth physics live on KH2's field run.** The tester (Missing sleeves animation m19): "lets do "another option" ... I can see how it looks in game before a baking starts", "3. ... the chain that sora has on his left hip", "4. physics only on the move", "start with soras running animation without the keyblade". New `kh1_data::clothsim` (shared with `examples/cloth_sim.rs`, so a later offline bake gives the same result) moves the coat (72, 248–279), shorts panels (26–41, 52–67), belt chain (10–15) and belt front (5–9): rest = KH1's field run (slot 5) averaged, a damped spring on the carrier's acceleration, lengths kept, swing caps, t569's clearance. `clothlive` runs it only on our slot for KH2's field run: baked, on KH1's own drawing of the bake (`+0x1B4` read back, changed bones written, `pose::cloth_on_kh1`); live, on our pose. One step per 60 Hz tick (`takeover::TICKS`). Tuned against KH1's run (stiffness / damping / weight / cap): coat 0.02/0.30/0.25/40°, shorts 0.20/0.45/0.25/30°, belt chain 0.06/0.05/0.25/100°, belt front 0.06/0.45/0.50/60°; swing 8.5 / 3.9 / 38 / 18° against KH1's 8.6 / 3.7 / 36 / 18. Checked (files). Assumptions: stride phase not matched (KH1's IK legs not evaluated); swing caps ours. Test `local_clothsim`. docs/kh1-secondary-motion.md "Cloth physics".

### 2026-10-25: Stage 591 (testing branch: Finishing Plus on KH1's own finishers)
- **Finishing Plus on KH1's own finishers.** The tester (m22): "you need to make sure every kh1 finisher can go into other kh1/kh2 finishers". While KH1's own finisher (plain or ability) plays at the KH1 setting with a finisher left, `combomods::kh1_finisher_window` opens KH1's combo window (instance `+0x14` bit `0x400000`) at KH2's 155 / 183 Finishing Plus window's share of the motion (`+0x16C` / `+0x280`, the share KH1 computes in `0x14029FF80`, Checked (decomp)); the walk picks the next one, and after KH1's own normal finisher the next normal one is KH2's by its walk (`0x140402C50`, Checked (decomp)): 154 with a foe in its window else 155, air 184 / 183 (`Character::normal_finisher`, sim test `normal_finisher_by_kh2s_walk`). Assumption: the window placed by share (KH1's finishers have none of their own). Single-target order by how far each moves the foe (docs only): Gravity Break 0 (`0x1402B7DE0`), Zantetsuken up to 80 (flinch push `0x1402B7E60`), Blitz launch (`0x1402B8530`), Checked (decomp, files). docs/kh1-anim-slots.md.

### 2026-10-25: Stage 590 (testing branch: Locked on, group checks count every enemy)
- **Locked on, group checks count every enemy.** The tester (m22 answer 4): "(a)". KH2's selector `0x140402D20` runs its kind-2 pass `0x1404029F0` on every target the iterator `0x1403BDAC0` gives on an attackable actor; the lock plays no part, only kind 1 uses the primary target (Checked (decomp)). `Character::row_targets` (the Base / Aerial Base Combo KH1 walk, `row_in_range`, `ability_row`) no longer keeps only the locked foe; the primary target and aim stay the locked foe's. KH2-setting selector (`try_attack`, stage 207's lock filter) unchanged. Sim test assertions added.

### 2026-10-25: Stage 589 (testing branch: Zantetsuken back, its KH1 rule)
- **Zantetsuken back, its KH1 rule.** The tester (m22 answer 3): "bring back zantetsuken and make it b as well". No longer taken out of Sora's list (stage 153's rule and the reward-line hook `0x1401B6990` are gone); a copy taken out before is given back once, off (`kh1-kh2-removed.txt`). KH1's `0x1402A6EE0` takes 0.3 off one random number per press for each equipped finisher in the order Stun Impact, Gravity Break, Zantetsuken (rows 3 / 4 / 5); row 8 only if all fail (Checked (decomp)). The walk holds bit 29 back and rolls Zantetsuken itself on that shared roll, always (its own rule, whatever KH1 Ability Chance says), in the single-target group after Gravity Break, before Blitz. Row 5 data (btltbl `0x4FD8`): gravity, power 250, reaction 3 (Checked (files)). Not checked: where KH1 grants it. docs/abilities.md, docs/kh1-anim-slots.md.

### 2026-10-25: Stage 588 (testing branch: Hurricane Blast keeps KH1's rule)
- **Hurricane Blast keeps KH1's rule.** The tester (m22 answer 3): "(b)". KH1's air picker `0x1402A6C70` (Steam, Checked (decomp)) marks row 15 only in state 2 at the finisher, with a target whose lock point (`0x1402B04C0`) is no more than 100 above him (y down), its request not the one playing. Bit 25 is no longer held back (`kh1picker::HELD_AIR` 0); the walk takes Hurricane Blast in Aerial Finish's place only when KH1's picker returned 15 (`Kh1Rule::Kh1Test`, as Blitz); the plain finisher is then 16 (18 if 16 plays), Blitz's 8 (13 if 8 plays; it fell back to 6 before). docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 587 (testing branch: KH1 Ability Chance off by default)
- **KH1 Ability Chance off by default.** The tester (m22 answer 2): "(b)". `settings` default false, ini template `kh1_ability_chance = off`. An ini line already written keeps its value.

### 2026-10-25: Stage 586 (testing branch: Tweaks inside Combat)
- **Tweaks inside Combat.** The tester (KH2's combo row selection, m22 answer 1): "(b)". Tweaks is Combat's tenth row (after Magic), its list's parent Combat; KH1 in KH2 is back to Camera, Combat, Movement, Misc. A list that would reach below the help box (y 300, def `0x140546160`) is drawn with KH1's window scale (`+0x6C` bit 0, `+0xA0` / `+0xA4`, anchor top) at 300 / its bottom: Combat at 0.85, the glint re-aimed through it (`config::list_scale`, `glint_to`). Checked (docs/kh1-config-menu.md §5 layout). Assumption: 85% fits and reads well. docs/kh1-config-menu.md §6.y.

### 2026-10-25: Stage 585 (testing branch: KH2's moves on KH1's states are left only in KH2's free state)
- **KH2's moves on KH1's states are left only in KH2's free state.** Same ask (kh2-magic-and-reflect-native m9). KH2's swings on KH1's combo requests kept the KH1 template's own "may be left" point, where KH1's control step jumped or rolled him out. `kh2slots::Leave`: swings, our rows and casts get KH1's "may be left" event only from KH2's free state, so none; KH1's action flag clears at the motion's end (row runner `0x1402A68F0` and `combomods` queue with `0x40000000`, cleared by `0x14029F6E0`; Checked (decomp) KH1 Steam). Reactions keep KH2's free state or else the template's point (Assumption: some KH1 reaction idles are queued without the bit). `roll_free` uses range id 2 (36). `cast_cut_open` lets a jump or Square cut a KH2 cast on KH1's cast state only in a free state, never for Sora's; Attack still from the reaction-command window. docs/kh1-anim-slots.md "Leaving KH2's moves on KH1's states".

### 2026-10-25: Stage 584 (testing branch: Jump and Square leave a move only in KH2's free state (controller))
- **Jump and Square leave a move only in KH2's free state (controller).** The tester (kh2-magic-and-reflect-native, m9): "you can jump and dodge roll out of moves during its end frames (I think around the same frames as when you can attack?) so this needs to be looked into the steam c decomp so its exact." Steam KH2's input step `0x1403D6FC0` calls the jump handler only with `+0x18C & 0x1C` (range ids 0 / 1 / 2, set by `0x14040D810`) or in the landing (motion 5); the action button's guard, roll, Quick Run and glide test the same through `0x1403BFB30`; Attack (`0x1403D6CA0`) takes the combo window (0x400), the finisher window (`+0x194` 0x20) or a free state. Checked (decomp). Sora's swings, ability moves and casts have no free state; the landing's is 12, the roll's 36, the guard's 52, the flinches' 50 (Checked (files)). `Clip::free_at` / `first_free`; `Character::run_action` takes a jump or Square only there (landing jump at any frame); the roll is left from 36, not its fall check at 34; flinches no longer from frame 8. `Character::can_leave` replaces `can_take_command` in `takeover::kh2_jump_open`. docs/movement-model.md "Leaving a move".

### 2026-10-25: Stage 583 (testing branch: Reflect's voice by Battle Quotes)
- **Reflect's voice by Battle Quotes.** The tester (kh2-magic-and-reflect-native, m7 answer 3): "if battle quotes is kh2 then (a) for kh1 we keep the grunts but the finisher will say "defence!" its one of the lines he picks when casting aero". `reflect::cast_voice` (both cast paths): at KH2, KH2's cast start's lines (Steam KH2 `0x1403C5170`: MAGC `+0x34` voice 7 on a cast, an even pick of five clips; `+0x35` voice 25 "Protect!" on the finisher; Checked (decomp, files)), through KH1's voice channel, both loaded at start. At KH1, the swing grunt (group 4) on a cast, and on the finisher Sora's KH1 Aero clip 46 at priority 3 through KH1's clip play (Steam `0x140290930`). Aero's group 13 has clips 46 / 47 (Checked (files)); Assumption: 46 is "Defense!", logged with its clip. docs/kh1-reflect-spell.md, docs/kh1-voice.md.

### 2026-10-25: Stage 582 (testing branch: Reflect stops enemy shots like KH2)
- **Reflect stops enemy shots like KH2.** The tester (kh2-magic-and-reflect-native, m7 answer 2): "(a) it needs to work exactly as kh2"; m9: "if you need to add more features into kh1's engine to get reflect working then do it". KH2's meeting `0x1403D2140` marks a shot's attack over (flag `0x200`) and tells its script; the barrier stays and nothing is sent back (Checked (decomp, KH2)). On Steam, `combat::stop_shot` replaces KH1's clash half `0x1402B4BC0` (which reflected free shots): the shot gets KH1's struck-no-damage result 4 (`+4`, `*(+0x60)`, as `0x1402B4060`), its hit label runs as when it strikes an actor (`0x1402BCC00 (process +0xA0, label +0x38, 1, &Sora's handle, 0)`, as `0x1402BF330`), and it is switched off (activity `0xC` cleared, as `0x1402BBC10`) so KH1's attack pass `0x1402B47A0` frees it. Checked (decomp, exe). Team and owner unchanged, still counted for the counter, contact effect still plays, melee unchanged (t573), Epic unchanged. Assumption: each enemy's hit label is its own reaction (not read per enemy). Logs "Reflect barrier stopped enemy shot ...", "... still flying 10 frames after meeting the barrier". docs/kh1-reflect-spell.md.

### 2026-10-25: Stage 581 (testing branch: Config: Tweaks list, KH1 Ability Chance)
- **A Tweaks list with KH1 Ability Chance.** The tester (combo row selection m21): "lets make a toggle in the config for the 30 percent chance to exist, if it does then if the 30 percent chance roll fails then it picks a similar kh2 move. these will be in the gameplay config in a sub menu called Tweaks". KH1 in KH2 gets a fifth button, Tweaks (`config::List::Tweaks`, the same KH1 window definition copy and back-out as the other lists). Its row KH1 Ability Chance (Off | On) switches `kh1picker::chance` live, saved as `kh1_ability_chance` (default on). Assumption: "gameplay config" = the KH1 in KH2 list. docs/kh1-config-menu.md §6.y.

### 2026-10-25: Stage 580 (testing branch: Combo: one finisher walk in KH1's picker (KH1 rule set))
- **KH1's and KH2's ability finishers in one walk in KH1's picker.** The tester (combo row selection m21): "do 1 ... you need to keep blitz's rule", "gravity break and blitz last, if its blitz and gravity equipped then blitz last"; m20: "(a) like kh2", "for explosion it should come last, for hurricane blast it should come first". `kh1picker` holds back Sora's ability bits 26 / 27 / 28 (ground) and 25 (air) (Steam `0x142D5EC10`, set by `0x1402A6380`, Checked (decomp)) while KH1's pickers `0x1402A6EE0` / `0x1402A6C70` run; KH1 still decides its count, length, dash, drop, Zantetsuken and Blitz (its own test, row 11's count, `0x1402A7320`, returns row 6). At the finisher `combomods::finisher_walk` goes through KH2's PTYA order (Checked (files); KH2 `0x140402C50` takes the first passing row, no chance, Checked (decomp)), each group gated by its KH2 row's range checks (`Character::row_in_range`): ground 26 [Stun Impact, Ripple Drive, Explosion]; 2 [KH1's finisher]; 27 [Guard Break, Gravity Break, Blitz]; 42 [Stun Impact, Ripple Drive, Explosion]; KH1's finisher; air 49 [Magnet Burst]; 7 [KH1's finisher]; 29 [Hurricane Blast, Aerial Finish]; 50 [Magnet Burst]; KH1's finisher. KH1's moves need their bit and KH1's anti-repeat; Stun Impact / Gravity Break take KH1's 30% roll with `kh1_ability_chance` on (KH1's random number `0x14031BA30`, prologue Checked (exe)), a failed roll moving on; none of KH1's moves when its picker took its dash (row 1). KH2's moves use `Character::ability_row_id`. Assumptions: KH1's finisher in the 154 / 184 place; Zantetsuken left to KH1; Hurricane Blast on Aerial Finish's checks. Logged ("KH1's ground/aerial finisher walk"). docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 579 (testing branch: Effects: Fire's hit area at KH1 Sora's size)
- **Fire's hit area at KH1 Sora's size.** The tester (m20, answer 3): "(a)". Fire's KH2 attack shapes (base Sora's KH2 Fire and Wisdom's) go through Reflect's barrier scaling, renamed `reflect::kh1_sized_shapes`: offsets from his place, radius and height x `pose::kh1_size` (0.89), matching Fire's effects since t561. Checked (code): the same path as t518. Blizzard, Thunder and Cure keep KH2's size. `docs/kh2-effects.md` §18.22.

### 2026-10-25: Stage 578 (testing branch: Magic: Air Physics KH2 holds KH2's hover on KH1's air cast animation)
- **Air Physics KH2 hovers him like KH2 on KH1's air cast animation.** The tester (m20, answer 2): "(b) hold him like kh2, the player expects it to be like kh2". `shared::physics::kh1_cast_hold`: with Air Physics KH2, while KH1's own air cast animation plays, `air_step` holds him in the no-gravity range of KH2's air cast clip for that spell (`kh2slots::CASTS`: Fire 58 16..74, Blizzard 61 16..66, Thunder 64 / Cure 67 16..58; Gravity on Thunder's, Stop and Aero on Cure's; Checked (files) P_EX100.mset), timed in 60 Hz frames from when KH1's air cast request began, KH2's arc clock running through it. KH1's air cast motions (slots 80 / 81: 78 / 68 frames, Checked (files)) outlast the holds. Air Physics KH1 unchanged. `docs/kh1-magic-native.md` §4i.

### 2026-10-25: Stage 577 (testing branch: Magic: KH1 spells on KH2's cast animation cast on its cast frame)
- **KH1 spells on KH2's cast animation come out on KH2's cast frame.** Same ask ("and vice versa"). KH1's script-call table entry 5 (`0x140529180 + 0x50`; original `0x1402B98E0`, Checked (exe); read through `0x140529FA0`) points at `formmagic::script_step`, which calls KH1's and, only for that cast's script process (the task's `+0x38` from the spell start `0x14028F140` / `0x14028F5E0`, matched with the running process `0x142DB7B48`, Checked (decomp)), scales the returned frame by KH1's cast frame over KH2's (the clip's magic-cast event) until KH2's cast frame. KH1's wait then ends on KH2's frame; its check, shot, MP and effects stay KH1's (ground Blizzard 28 on 16, Thunder 30 / Gravity 28 on 18, Cure 36 / Stop 28 / Aero 30 on 18; air 28 on 36, 30 on 30). Steam only. Assumption: +-1 frame. `docs/kh1-magic-native.md`.

### 2026-10-25: Stage 576 (testing branch: Magic: KH2 spells on KH1's cast animation, released at KH1's cast frame)
- **KH2 spells on KH1's cast animation come out on KH1's cast frame.** The tester (kh2-effects-in-kh1s-own-effect-system m20, answer 1): "if a kh2 spell is casted on a kh1 animation then it should cast on the frame that the kh1 animation casts it and vice versa". `formmagic::anim_kh2` follows each spell's Animations row for KH2's Blizzard, Thunder and Cure too (Fire has no row). At KH1, `kh2slots::redirect` leaves KH1's cast slot on the request and `kh1_spell_start` starts KH2's script; `shots_step` holds the script Sora in KH2's cast motion until KH1's frames since the start (frame scale `0x14233FBDC` per physics frame) reach `KH1_CAST_FRAME` (Blizzard 28, Thunder 30, Cure 36), or KH1's applied cast request `+0x164` / action flag goes. Checked (files): KH1's `xm_*0N.mag` scripts, all levels, read through KH1's script machine `0x1402CC620` (Checked (decomp)) and call table `0x140529180` (Checked (exe)). Assumption: +-1 frame from task order. KH2's cast motion's own frame events don't play on KH1's animation. Config help reworded. `docs/kh1-magic-native.md` §4j, `docs/kh1-move-styles.md`.

### 2026-10-25: Stage 575 (testing branch: Glide: bank changes logged (KH2's restart rule checked))
- **Glide bank changes: KH2's restart rule checked, each change logged.** The tester (m6): "when sora turns to another bank after hes banking in a direction he resets his animation which doesn't seem like kh2." KH2's glide update `0x140423A70` asks `0x1403C6DC0(holder, 205/206/207, 16, 0)` every frame; a motion not already playing starts at time 0 with a 16-frame blend, play time carried only between walk and run (Checked (decomp)). 205..207 are 30-frame loops (A170..A172, Checked (files)). KH1's request `0x14029F7C0` / apply `0x1402A0350` do the same, with a mid-blend path `0x140393B30` from the `+0x1B0` channels (Checked (decomp)). So the restart is KH2's rule; no change to the animation. Each glide motion change is logged (first 80) with KH1's slot, time and blend. Not checked: KH2's blender mid-blend (`0x1403C88C0` / `0x1403C8CD0`). `docs/kh1-glide-native.md` §8.9.

### 2026-10-25: Stage 574 (testing branch: Glide: the ability list follows the Glide style at once)
- **The Glide style swaps the ability list the moment it's changed.** The tester (KH2 Glide thread, m6): "glide and super glide don't get removed anymore when setting glide to kh2, and kh2's glide growth ability isn't there." KH2's own Glide (id 88) replacing KH1's Glide and Superglide in the shared list (`growth::glide_swap`, t323) only ran once a second from `menu::Growth::update`, which only Sora's physics frames call, so after switching the style in the paused Config menu KH1's Abilities screen still showed KH1's two. Now `config::set` for the Glide row calls `menu::glide_style_now` at once (the swap, KH2 Glide's name with its level, a log line "abilities: Glide style ... set in the Config menu"); KH1 rebuilds Sora's ability bits `0x142D5EC10` on the menu's close (`0x1402A6380`). Checked (code); Checked (logs): the swap itself works in t544 and t567. Assumption: this timing is what the tester saw. `docs/kh1-glide-native.md` §8.8.

### 2026-10-25: Stage 573 (testing branch: Reflect: hit outside the barrier; melee meets it as KH2's guard)
- **Reflect: untouchable only while a barrier stands; melee clashes with it.** Same ask (m6 answer 3). KH2's 71 / 72 have no invulnerable range (range id 27, Checked (files)); the same is taken for 73 (Assumption from §4e). Sora's untouchable flag 0x40 is set only while a barrier stands, and `hurt` no longer evades hits through the cast (undoes main's s179 / s226). An enemy melee attack meeting the barrier gets KH1's clash with a guard, so its owner recoils, as KH2 stops the attack and plays the owner's clash reaction (`0x1403D2140`, Checked (decomp)). `docs/kh1-reflect-spell.md`.

### 2026-10-25: Stage 572 (testing branch: Reflect: KH2's script as it is (counter re-hits, burst, every cast its own))
- **Reflect's counter and burst as KH2's script runs them.** The tester (m6 answer 3): "reflect is currently not 1:1 with kh2, from how it falls to how it behaves in game, please look at the steam c decomp". Checked (files): ATKP 573 kind 3 (special guard), power 0; REFLECT_1.mag through kh2-magic: barrier from script frame 2 for 27 frames; no hits, no counter; with hits 574 / 576 / 578 (power 50, flinch, re-armed every 10 / 8 / 6 frames) 30..57, then the burst 667 / 668 / 669 58..63. Checked (decomp, KH2): the barrier meeting `0x1403D2140` counts on the barrier's own attack `+0xB0`. Every script attack strikes with its own row (undoes main's s229 / s233 single pop); a second Reflect leaves the first running with its own barrier, count and counter (undoes s175 / s182; Assumption: KH2 never cancels a running Reflect); hurt's blocks go to the newest barrier; the burst sound (element 7) on 667..669's first frame. `docs/kh1-reflect-spell.md`.

### 2026-10-25: Stage 571 (testing branch: Reflect in KH1's flight cast)
- **Reflect in Neverland flight on KH1's own flight cast.** The tester (m6 answer 2 (a)). KH1's flight cast `0x1402A4150` (Checked (decomp)): usable check; with a target in the command record (`+4` handle, `+8` lock point) aims the move direction at it (`0x1402B04C0`, `0x140386D10`) and sets stats `+0xD0` to pi/16; the air request `| 0x1000000` with KH1's flight motion queued (`0x40000075`, blend 8.0); the spell start; no hop, no halving. `reflect::native_ok` takes state 8 and `cast_native` does those steps with our air request (motion 73). Drive Forms not moved (a form still runs on KH2's controller, no KH1 cast state under him). `docs/kh1-magic-native.md` §4f.

### 2026-10-25: Stage 570 (testing branch: MP: KH2's cost kinds in KH1's engine (Cure: all MP, one bar))
- **KH2's MP cost kinds in KH1's own MP check, payment and menu.** The tester (kh2-magic-and-reflect-native m6, answer 1): "(a) based off of the config; we need to give kh1's menu and mp the ability to allow costs like kh2 cure ... it wipes your whole mp bar. this needs to be a feature in the engine ... at least 1 bar of blue mp before you can cast kh2 cure". KH1's MP check `0x1402A4C10` and payment `0x1402A5B60` (hooked) take two new kinds beside KH1's (units under 100, bars x 100): `0x40000000 | 1` all MP with at least one whole bar (KH2 Cure, the tester's bar rule), `0x20000000 | percent` a share of max MP. KH1's usable check `0x14028EFD0` is hooked: for base Sora's Fire..Cure with their Magic row at KH2, the spell data's `+0x30` reads the KH2 kind for that call only, so the Magic menu fill `0x1402944C0`, the shortcuts and the command step grey and refuse by KH2's cost (KH1's at the KH1 row). Our KH2 casts pay through KH1's payment. Checked (decomp): KH1's check, payment, usable check and callers; KH2's `0x140401D60` / `0x1403D88E0` / payment `0x1403D3CF0`. Checked (exe): the three prologues. Difference by the tester's choice: KH2 lets Cure (and short spells) cast on any MP. Assumption: the per-command flag `0x1403C5940` / `0x1409ACFF4` unidentified. `docs/kh1-magic-native.md` §6, `docs/kh1-kh2-mp.md`.

### 2026-10-25: Stage 569 (testing branch: Hood, coat and shorts from KH2's hidden keys (test bake: walk and run))
- **Hood, coat and shorts from KH2's hidden keys, test bake in the walk and run.** Mr Bugs (Missing sleeves animation), after t552: "sora's hoodie still stays rigid ... it needs to look natural and his clothes not clip"; he picked option (c): "no it should be like kh2" (no lag or settling), "(a) just do a test bake into soras running and walking animations for now". New `kh1_data::cloth::ClothBake` runs in `pose::Baker::poses` only for KH2's walk (1) and run (2), battle and field (`CLOTH_MOTIONS`), after the retarget and `Secondary`, over the whole motion: hood 80-83 from KH2's hood keys 72 / 73 against the chest 68 (as at t375, without `KH2_SHOWN_ON`); coat from `coat_kh2` as now; shorts 27/31/35/39 and 53/57/61/65 sway as KH2's trouser leaves on that thigh sway about their mean, blended by place round the thigh. Ours, not KH2's: centred Gaussian smoothing (sigma 1.2 frames, wraps the loop, no lag) and clearance (hood 7 off the spine, 10 off the head; coat outside the shorts, at most 5 nearer the hips' axis than in the bind; shorts at 0.95 of their bind distance), cleared by a swing about one axis fixed per motion (no flips), spread over +-3 frames. Live poses and other moves unchanged; one log line per bake. Checked (files): KH2's keys and binds, all four motions (`examples/cloth_check.rs`, test `local_cloth`). Assumption: which trouser bone stands for which KH1 panel; the shorts' radius (from bones, not the mesh). `docs/kh1-secondary-motion.md`.

### 2026-10-25: Stage 568 (testing branch: Battle quotes: KH2's line cut through KH1's voice channel)
- **KH2's rule for Sora's battle quotes: a new line at least as important cuts the one playing.** The tester (KH2 sounds thread, m5 question 1: "(b)", then "do your recommendation, go ahead"). KH2: the quote request `0x1403CE580` takes a line when nothing is pending or playing, or when its priority is at least the last one taken; its voice step `0x1403CE260` stops the playing handle and starts the new line (Checked (decomp, KH2)). KH1's voice channel keeps no priority (`0x140290930` / `0x140290080` compare only a standing; Checked (decomp)), and `0x1400DE010 (2)` pauses effect and voice records rather than stopping them (same call as mode 1, `0x140056860` / `0x140056890`; that it is a pause is an Assumption from the pairing). KH1 ends a record with the engine stop `0x1400565A0 (sound, &0)`, as `SslSeStop` `0x1400DEF50` and the frame clean-up `0x1400DC8C0` do (Checked (exe, decomp)). Now `kh1bank::voice`, when Sora's channel (`0x142D371D0`) holds one of our KH2 quotes: a lower line is dropped; otherwise our bank's state-3 records in the live list `0x1404D6618` get the engine stop at fade 0, the channel goes to state 0 and KH1's own clip request takes the new line; KH1's clean-up deletes the stopped records. KH1's own lines keep KH1's rule. Logged "KH2 quotes: clip ... cut for clip ..." / "... dropped". Assumption until heard: the stopped record ends at once. `docs/kh1-sound-native.md`.

### 2026-10-25: Stage 567 (testing branch: Battle quotes: KH1's swing voice held back where KH2's quote plays)
- **KH1's swing grunt no longer takes Sora's voice channel ahead of KH2's quote.**
- Found in the tester's t544 logs (Battle Quotes KH2): every KH2 quote asked of KH1 was refused ("KH1 kept the line playing", all of them).
- The cause:
  - KH1's attack row start `0x1402A3980` ends with the row's `+0xF` voice through the voice group `0x1402901F0`. The call is at `0x1402A3AAF` (return `0x1402A3AB4`). Checked (exe).
  - Sora's rows carry one: air 16 / 17 / 18 have 5 / 4 / 4, ground 13 / 14 have 4 (`btltbl.bin` `+0xA9F0`). Checked (files).
  - The group then holds the channel for Sora (standing 2). KH1's clip request `0x140290930` only asks when the standing is below the new priority, so KH2's priority-1 / 2 quote never got in. Checked (decomp).
- Now, with Battle Quotes KH2 and KH2's voices loaded, `soundlog`'s voice-group hook leaves that call out on Sora when the row's request plays a KH2 motion of ours.
  - The call is told by its return address.
  - The row is read from combat state `+0x40`, which the row start writes just before. Checked (decomp).
  - The request comes from Sora's row table; `kh2slots::plays_kh2` = KH2's row selection picked a motion for it.
  - The KH2 motion's own quote takes its place, as in KH2, where a swing has only its own line.
- KH1's other voices of Sora, and rows that play KH1's own motions, are unchanged. Logged ("KH2 quotes: KH1's swing voice of row ...", first 20).
- Open: KH2's own priority rule. KH2 `0x1403CE580` / `0x1403CE260` take a line of priority at least the last taken and stop the playing one (Checked (decomp)). KH1's voice path never stops a line, so while Sora's last KH2 line plays the next is dropped. Asked in the thread.
- `docs/kh1-sound-native.md` "KH1's swing voice and KH2's quotes".

### 2026-10-25: Stage 566 (testing branch: KH2 sounds: one copy of each bank (RAM))
- **One copy of each KH2 sound bank.** This answers the tester's RAM question from m4 ("are we caching a bunch of sounds?").
- KH1's bank-entry destructors (`0x1400DB470` / `0x1400DB7A0`) free the entry's `+0x20` with `_aligned_free` only when it isn't 0. No KH1 function that walks the bank list `0x1404D65D8` reads `+0x20` / `+0x28` afterwards. The engine keeps its own pointer (bank `+8`, `0x14005E0B0`). Checked (decomp).
- So `kh1bank` now:
  - makes each bank's buffer once, with KH1's `_aligned_malloc`;
  - drops our `Vec` at that point;
  - sets the entry's `+0x20` to 0 right after KH1 takes it;
  - registers the same buffer again after each KH1 bank clear.
- KH1's engine writes into a bank as it loads it: ActualBank set-up `0x1400748F0` (vtable `0x140428790` entry 2) sets byte `+0xF` of each sound record to 0xFF. Checked (decomp, exe).
  - `scd_cut::as_loaded` writes the same up front, and again before each re-registration.
  - The buffer is then checked against the hash taken at the cut. A changed bank is logged and left out.
- Assumption: nothing else in an ADPCM bank changes during play. The log will say.
- RAM: ~9.0 MB (one copy) plus the voice bank's 0.6 MB. Before this round it was ~77 MB; before t509, ~58 MB.
- The voice bank keeps KH1's copying registration (`0x1400E0110`).
- `docs/kh1-sound-native.md` "One copy of each bank". Test `as_loaded_marks_each_sound_record`.

### 2026-10-25: Stage 565 (testing branch: KH2 sounds: only through KH1's engine (the old player removed))
- **KH2's sounds go only through KH1's own sound engine; the mod's player and its decoded audio are gone.** The tester asked: "I want you to gut the old system and transfer everything ... get it fully working."
- `kh2sound.rs` no longer decodes anything. The loader reads each KH2 bank, cuts the wanted sounds into one-sound banks (`scd_cut::cut`), hands them to `kh1bank` and frees the whole banks. That removes ~56 MB of decoded effects plus the decoded quotes.
- The battle-quote clip list keeps the clips whose record has audio.
- Removed:
  - the `waveOut` player (`play` / `tick`, the PCM copies, `sys.rs`'s winmm imports);
  - its volume read (`game_volume`, KH1's settings block);
  - the ini's `kh2_sound_engine` and every "mod's player" fallback.
- A sound KH1's engine can't take at that moment isn't heard, as KH1 drops its own. That covers a bank not yet back after a room load, the queue full, or a voice line KH1's priority rule keeps. The first 20 are logged ("KH2 sound not played: ...").
- Our effect slot in KH1's queue (`0x140179AC0`) is still stopped with KH1's `0x140179D90` once its length has played and `0x1401789F0` says it's done.
  - The length now comes from the audio header (`scd_cut::length_ms`: Microsoft ADPCM blocks x samples per block).
  - A stream with no length in its header is stopped once KH1 has said it plays and then that it doesn't (2 s if it never starts, 30 s at most).
- Off Steam nothing is registered, so no KH2 sound is heard there.
- Checked (code).
- Checked (files): every effect stream in se999 / se000 / se500 is codec 12 (ADPCM); the voice bank is 31 Vorbis streams. The lengths match the decoder's own block math.
- `docs/kh1-sound-native.md` "Only KH1's engine". Test `adpcm_length_from_the_header`.

### 2026-10-25: Stage 564 (testing branch: Magic: Air Physics row)
- **Air Physics: KH1 | KH2 for every air cast.** The tester (same thread, m19: "change
air magic to air physics kh1 | kh2 so now you can cast kh2 fire with kh1 air physics like how we
did for reflect. remove anything related to the main physics setting that has magic in it and put
it in this new air setting"). Config > Combat > Magic > Air Physics (ini `magic_air_physics`, its
default the old `physics` value), `shared::physics::cast_rule`. Moved off the Physics row:
`cast_stops` (KH2's 40 hop and kept speed, or KH1's own 30 hop and half speed), `air_step` while a
cast plays in KH1's air state (new `casting_in_air`: action flag + KH1's air cast request 0x83..0x8A
at `+0x164`, or a cast slot of ours; KH2's arc / gravity / drift / no-gravity hold with the clock,
or KH1's step), and the KH2 controller's borrowed air casts (`takeover`'s `air_magic_falls`). New:
KH1's own spells in the air take the row's start too (`formmagic::spell_start` calls `cast_stops`
for them). A cast at Air Physics KH1 under Physics KH2 drops our arc so the fall after it goes on
from KH1's fields. Physics row help text points to Air Physics. Checked (decomp) for the KH1 / KH2
air cast numbers (t534's §4g); Checked (files) the request range. Assumption: KH1's halving and 30
hop come after the spell start in `0x1402A3600` (what §4g already relies on). With a KH1 cast
animation (no KH2 slot) there is no KH2 hover at Air Physics KH2 (asked). `docs/kh1-magic-native.md`
§4i, `docs/kh1-move-styles.md`.

### 2026-10-25: Stage 563 (testing branch: Magic: KH1's Gravity, Stop and Aero on KH2 cast slots)
- **Gravity, Stop and Aero on KH2's cast animation, natively.** The same ask (their
animation rows at KH2). Before, KH2's controller borrowed Sora for them and KH2's cast motion was
drawn over KH1's cast on its clock (`takeover::command_started`, `pose::follow`,
`BASE_KH1_CAST`). Now `kh2slots::CASTS` adds our slots of KH2's Thunder cast (62, air 64) for
Gravity and Cure's (65, air 67) for Stop and Aero on KH1's own requests 0x3A / 0x3B / 0x3C and air
0x87 / 0x88 / 0x89 (KH1's request table: slots 55 / 56 / 56, air 80 / 81 / 81, Checked (files)
`xa_ex_0010.mset`), and `combat::command` lets KH1's command step cast them there with KH1's own
script, cost and effects (`kh1_on_kh2`, KH1 states 0 / 6 / 0x18 / 2 / 8). The borrow is left only for
a state KH1 casts nothing in. KH2's cast windows (cut / combo, `cast_follow`) apply on them. Six more
baked slots at load. `docs/kh1-move-styles.md`.

### 2026-10-25: Stage 562 (testing branch: Magic: a Config row per spell and its animation)
- **Magic's rows per spell.** The tester (same thread, m19: "each magic should have its
own settings of which magic you can choose, e.g fire: kh1 | kh2 and that would change what gets
casted, we'll remove ground magic ... you can also change the animation of magic with the exception
being reflect, and fire"). Config > KH1 in KH2 > Combat > Magic is now Fire, Blizzard, Thunder,
Cure (what is cast, ground and air alike; ini `magic_fire`, `magic_blizzard`, `magic_thunder`,
`magic_cure`) and Animations, its own list (Blizzard, Thunder, Cure, Gravity, Stop, Aero: the cast
animation of KH1's spell; ini `anim_blizzard` .. `anim_aero`), so both lists keep KH1's full-size
rows on screen (config-menu-rework answer 1 (a)). The Magic and Aerial Magic rows are gone; the old
`magic`, `cure_style`, `kh1_cast_*` keys are read only as the new keys' defaults (an ini from before
casts what it did). `formmagic::kh2_spell` / `base_kh2_spell` / `anim_kh2` replace `base_kh2`; a
spell cast KH2's way always plays KH2's cast motion (its script needs it, §4 of
docs/kh1-magic-native.md); Fire's animation is its spell's, Reflect's KH2's. KH1's Blizzard, Thunder
and Cure with their animation row at KH2: KH1's requests 0x37..0x39 / 0x84..0x86 point at our KH2
cast slots (`kh2slots::redirect` by the new `cast_spell`), KH1's own script runs on them (no
borrow). Gravity / Stop / Aero at KH2 as before in this version (borrowed, drawn on KH2's motion).
The debug menu's KH1 Spells page now shows and sets the Gravity / Stop / Aero animation rows. Trace
CSV: the config field is 32 bits, columns `cfg_fire` .. `cfg_anim_aero` added (`cfg_magic` /
`cfg_air_magic` are the old keys' values, no longer used). Checked: the code paths (combat::command,
kh1_spell_start, redirect). Assumption: KH1's spell scripts time their shots by their own frames,
not by the cast motion (KH1's cast motions carry no spell events, Checked (files)), so KH1's spell
comes out at KH1's time on KH2's animation. `docs/kh1-move-styles.md` Magic.

### 2026-10-25: Stage 561 (testing branch: Effects: Fire at KH1 Sora's size)
- **Fire's KH2 effects at KH1 Sora's size.** The tester (kh2-effects-in-kh1s-own-effect-system
m19: "1.for fire do (a)", answering m18's question 1, "(a) scale every KH2 spell and move effect on
Sora to KH1 Sora's size the same way"). `kh2dpx::kh1_sized`, which scaled Reflect's packages since
t483, now also takes Fire's three packages (`formmagic::spell_package(0, level)`, base Sora's KH2
Fire and Wisdom's): each element's scale (KH1's instance scale, which carries its header offset:
the ring, its flame trails and the Keyblade pieces, places and sizes) and its position-only offset
x `pose::kh1_size` (0.89, the two bind poses' hip ratio). Fire's attack shapes stay KH2's size;
Blizzard, Thunder, Cure unchanged. Checked (decomp): KH1's instance scale `0x14031CEA0` (t483's
check). Assumption: none of Fire's pieces is placed by its script (`spell_place`, not scaled).
`docs/kh2-effects.md` §18.22.

### 2026-10-25: Stage 560 (testing branch: Hit effects: KH2 swings call the Kingdom Key's KH2 hit spark)
- **KH2 swings' Keyblade hits show KH2's own hit element, through KH1's lookup.** The tester (m2 answer 1: "when you hit an enemy an effect spawns on the enemy (keyblade hit effect, fire hit effect etc.) Please look into it and find an implementation thats native, I give you the go ahead"). `combat::swing_spark`: a hit of a KH2 swing's right-Keyblade shapes (weapon 1) by base Sora with the Kingdom Key (0x51, save `+0x36`) lands with `Kh2Spark { Pkg::Weapon, effect_on_hit, random_effect }` (Kh2Spark now carries a `kh2dpx::Pkg`) through t521's path: attack `+8` = the Kingdom Key's KH2 package (`W_EX010.a.us`), data copy `+0x28` = effect_on_hit − 1; KH1's contact (`0x1402b4060` → `0x1402b4320` → `0x1401dffe0` → `0x1402ae650`) picks, places and turns it. `kh2dpx::WEAPON_ON` builds hit elements 1 / 2 / 3 (kinds 1, 1, 0x102; Checked (files)); pppKeBornRnd5 (183) joins `KH1_SAME` (KH1 `0x14033E3D0` / `0x14033E4B0` / `0x140349EA0` vs KH2 `0x1404D4EA0` / `0x1404D4F80` / `0x1404E1780` decompile the same but the paused offset; constructs `0x14033E490` / `0x1404D4F60` same bytes; Checked (decomp), Checked (exe)). ini `kh2_hit_sparks` (default on). KH1's own swings, other Keyblades (answer 1 (c)), body shapes, Drive Forms and the hit sound unchanged. KH2's damage_effect is the target's own set (`target + 0x80`, kind 0x200, Checked (decomp) `0x1403d1b40`): KH1's enemies have none, left out. Assumption: a swing's attack carries its weapon's PAX. docs/kh1-kh2-hit-effects.md §9.

### 2026-10-25: Stage 559 (testing branch: KH2 effects: KH2's 0x4009 point on KH1's object root (Cure))
- **KH2's 0x4009 point is KH2's own: the object root plus the kind-9 shape's centre, at KH1 Sora's size.** The tester (kh2-hit-effects-on-enemies: "reflect, it still seems off centred, before these changes it wasn't how were you attaching it to sora? can we do the same while keeping it native? same goes for cure as well"; effects thread: "kh2 cure doesn't seem to be positioned properly either"). Since t470 `kh2dpx` started 0x4009 on KH1's body centre (0x8005) plus 20: KH1's first kind-1 collision part, a point along one of his bones (`0x14029EB20` → `0x1402B6D30`), which sways with the cast; the t544 log has it 90 up and 10 to one side ("body centre: KH1's 90.0 above Sora's place (offset [10.0, -90.0, -0.8])", Checked (trace)). KH2's 0x4009 (`0x1403B50F0`, Checked (decomp)) is the kind-9 shape's centre on the object root (Sora's 105 up, Checked (files) `P_EX100.mdlx`), which no motion moves. Now: KH1's object root 0x8000 (its frame §18.23) with the element's offset moved by that point (`effect_point_of`), and `kh1_point` scales a position-only element's offset on it by `pose::kh1_size` (~0.89, as Reflect's): Cure's elements 3 and 4 (`CURE_n.mag`, flags 0x245, Checked (files)) at ~86 / 93 above his place, centred. Logged ("on KH2's 0x4009 point"). Reflect unchanged (all its pieces are on 0x4000; none on 0x4009). docs/kh2-effects.md §18.16.

### 2026-10-25: Stage 558 (testing branch: KH2-only combo abilities follow the combo settings)
- **Finishing Plus / Negative Combo no longer turn the combo to KH2's; at KH1 they act on KH1's own combo.** The tester (m19: "the ability should not ignore the combat settings"). `styles::kh2` is the setting alone (t488's override gone; the log line now says the ability follows the settings). Negative Combo: `kh1picker::shorten` lowers the stats' length byte (`+0xD4` / `+0xD5`) by its copies (at least 1, KH2's rule) only while KH1's picker runs, so KH1's own finisher test marks the finisher sooner; `combomods::kh1_combo` reads the same length (`kh1_length`). Finishing Plus: KH2's Finishing Plus window on a KH2 ability finisher playing on KH1's rows (`kh2slots::finisher_window`) opens KH1's combo window only while a finisher is left (1 + copies, `finisher_chain_open`); the counted press's finisher is KH1's picker's (a KH2 ability finisher by KH2's checks, via `next_finisher`, or KH1's own finisher row), on a row whose request isn't playing; none, no attack. KH2 setting unchanged. Checked (decomp) for the length byte's readers in the pickers; Assumption: nothing else reads the byte during the picker's call. Not applied to KH1's swimming combo (state 0x18) or the spell combo-finisher test (`formmagic`). docs/kh1-move-styles.md.

### 2026-10-25: Stage 557 (testing branch: KH1's row picker picks KH2's ability moves)
- **At Base / Aerial Base Combo KH1, KH2's ability moves come out of KH1's own row picker.** The tester (m19: "if an ability is equipped thats kh2s and your combat config is set to kh1 then you should be able to do kh1s moves, and its row picks, we need to modify kh1s so that it can choose explosion and other moves during its kh1 combo", "patch the extra functions that kh2 uses in kh1"). New `kh1picker.rs` wraps KH1's ground picker `0x1402A6EE0` and air picker `0x1402A6C70` (Steam only; prologues `4C 8B DC 55 56 57 49 8D AB 48 FF FF FF` / `40 53 55 56 57 41 54 41 56 41 57`, Checked (exe)). KH1's picker runs whole (its count, its length test at the stats' `+0xD4` / `+0xD5`, its own ability rows by `0x142D5EC10` and chances, its target, its anti-repeat; Checked (decomp)); where it took one of its plain rows (ground 1, 8, 11, 13, 14; air 16, 17, 18), `combomods::picker` adds KH2's ability rows as the next candidates: the first equipped ability row of the kind KH1's count is at, playable on KH1's rows, not the KH2 motion playing, passing KH2's own row checks (new `Character::ability_row`: KH2's `condition` and `in_range`, KH2 Steam `0x140402750` / `0x1404029F0`, Checked (decomp) in earlier stages). The picker returns the KH1 row it runs on and the row runner plays it as before. KH2's selector (its table walk, its length) no longer runs for the Attack combo at the KH1 setting; t537's filter is gone. Flight (state 8), Drive Forms and the KH2 setting unchanged. Assumption: KH2's ability rows go after KH1's own ability rows and before its plain rows. docs/kh1-anim-slots.md §7.z, docs/kh1-move-styles.md.

### 2026-10-25: Stage 556 (testing branch: Recoil KH2: refact-self-2 swings make a guard's owner immune)
- **KH2's refact self 2 against a guard, on KH1's meeting.** The tester (m2 item 2: "add it and include in the kh2 config for recoil"). KH2 `0x1403D2140` (Checked (decomp)): a normal refact-2 attack meeting a guard goes on and the guard's owner becomes immune to it. KH1: the pass `0x1402B3BE0` calls the meeting `0x1402B4A20 (entry, entry)`, which runs the directional tests `0x1402B4640` and the halves; our refact-2 rows (0x62+) against a guard (100) make both halves do nothing, so the swing hit the body behind. The pass skips a body whose `+0x3E0` holds the attack's `+0x54` bit (cleared on re-arm `0x1402B3380` / free `0x1402B4460`). New hook `recoil::meet` (Steam `0x1402B4A20`, prologue `40 53 56 57 41 56 48 83 EC 78`, Checked (exe)): after KH1's meeting, at Recoil KH2, Sora's attack of a refact-2 row (`kh1rows::unstoppable`) against an enemy guard (priority 100), both directional tests passing (`0x1402B4640`, bytes checked before the call): the guard's owner gets the swing's hit-once bit, with KH1's clash spark (KH2's clash effect 0xD). Recoil KH1: nothing. Assumption: KH1's pass tests the meeting before that frame's body. docs/kh1-kh2-recoil.md §6.9.
- 1:1 check: KH2 decomp (§1.2), KH1 decomp of the pass / meeting / halves / re-arm, exe prologue.
- Not checked: in game; the pass's entry order (above).

### 2026-10-25: Stage 555 (testing branch: Recoil KH1: Guard Break goes through guards)
- **Guard Break pierces guards at Recoil KH1 too.** The tester (m2 answer 1: "guard break should function like how it does in kh2, it should pierce any normal enemies"). At KH1 our Guard Break row (row 69) gets priority at least 0x62 (`kh1rows::GUARD_THROUGH`; KH1 adds the Keyblade byte, cap 99) in place of the template's: KH1's half `0x1402B4BC0` meets two attacks only when one is under 0x62 and never stops a side at 0x62+ (Checked (decomp)), so a guard (100) never meets it and it reaches the body. An enemy attack under 0x62 still meets it and is stopped, its owner recoiling by KH1's rule (kind 7 past a gap of 40): KH1's, not KH2's (KH2's pierce meets no normal attack). Recoil KH2 unchanged (type 6). Counterguard and Finishing Leap keep their templates' priority at KH1. docs/kh1-kh2-recoil.md §6.8.
- 1:1 check: decomp of the half's priority test.
- Not checked: in game.

### 2026-10-25: Stage 554 (testing branch: Recoil KH1: no KH2 guard windows in Sora's attacks)
- **At Recoil KH1 Sora's attacks carry no KH2 guard.** The tester (m2: "guard break and other kh moves still use the kh2s recoil mechanics when I set recoil to kh1"; his trace `trace_1792887810_f39547` and the t544 log: Guard Break's own Keyblade guard, ATKP 162 frames 5..42, went up as KH1 guard objects and `hurt` took the enemy's three hits on it, "KH2's guard on the Keyblade (ATKP 162, refact self 2), no effect"). KH1's swings carry no guard: an enemy attack meeting one is KH1's clash by priority (`0x1402B4BC0`), or it hits him (Checked (decomp)). New `recoil::guard_window_on(motion, weapon)`: at Recoil KH1, base Sora's Keyblade guard windows (KH2 weapon 1 / 2) guard only in the guard press 173 and its repel 42; Guard Break 165's and Vicinity Break 170's 162s don't (`combat::guard_step` puts no guard object, `kh2slots::keyblade_guard` gives `hurt` none). Body guards (weapon 0: Aerial Dodge, Flash Step, Aerial Recovery, Counterguard, Vicinity Break's 406) and Drive Forms unchanged; Recoil KH2 as before. Checked (trace), Checked (decomp). docs/kh1-kh2-recoil.md §6.7.
- 1:1 check: trace/log; decomp of KH1's half.
- Not checked: in game; whether the enemy then recoils against Guard Break (it should: V3's priority).
- Overlap: body guards are the bodyguard helper's (left alone on purpose).

### 2026-10-25: Stage 553 (testing branch: Recoil: the Guard Repel row merged into it)
- **One Recoil row for recoil, parry and the guard repel.** The tester (kh2-recoil-and-parry-system m2: "we need to also combine guard repel and recoil into the same config"). The Movement list's Guard Repel row (t536, ini `guard_repel`) is gone; `combomods::repel_kh2` is the Recoil row (`recoil::kh2`): what base Sora's KH2 guard does on a block is KH1's own block (`kh1_block`: 0x6E / 0x6F by the priority gap, 8 frames, KH1's Counterattack) at Recoil KH1 and KH2's repel (motion 42, 200 back, 80 frames, Counterguard) at KH2. `guard_repel` no longer read or written (its default was kh2; `recoil` defaults to KH1, so a fresh ini and the tester's now get KH1's block). Recoil row help reworded. Checked (code). docs/kh1-kh2-recoil.md §6.6.
- 1:1 check: code paths only (repel_kh2's two callers).
- Not checked: in game.

### 2026-10-25: Stage 552 (testing branch: Sleeves thread: the coat turns as KH2's coat ring)
- **The coat moves as KH2's coat ring.** The tester (missing-sleeves-animation, after t519): "when I mean by sticking out is that the coat is stiff and not animated at all. redo the coat so that its actually animated ... this is happening on all kh2 animations that we've baked so far" ("we'll fix the clipping later"). t491-t519 kept the coat rigid on the chest (t519: hem on the hips), so under KH2's moves it never moved, live or baked. KH2's coat ring (root / leaf pairs under the chest 68: 69/70 back, 128/129 .. 134/135 +x, 220/221 .. 226/227 -x; each leaf's bind axis within 7 degrees of KH1's matching panel) is keyed by F-curves only: at its bind relation in the field idle, hanging down from the leaning chest in the battle stance and swings, swaying 9-21 degrees in the idle and run and up to 50-120 in swings, jumps and hit reactions (Checked (files): `P_EX100.mdlx` / `.mset`, all 83 motions; the earlier "held flared out" was that hang measured against the chest). `retarget::coat_kh2` (live and baked, replaces `coat_low`): each KH1 chain's joint (73, 249, 253, 257, 261, 265, 269, 273, 277) takes its leaf's turn from its bind with KH2's chest turn taken out, on KH1's chest 78 (as the hair takes its spike's); roots stay on the chest, the joints below keep their bind relation, lengths kept, no guard. Measured: a tip swings 33 degrees against the chest in a motion (median, max 109; was 0); out >2 off the hips' axis 44.5% (KH1's own coat 15%), inside the shorts 1.19% (was 0.55%), tips past 90 degrees from their bind 1.0% of chain-frames (jumps, spins, reactions); the run's +x front tip 8-12 into the shorts' front. Test `local_own` (`kh1_s_coat_turns_as_kh2_s_coat_ring_under_kh2_s_poses`). Tool `examples/coat_ring.rs`. docs/kh1-secondary-motion.md "The coat follows KH2's coat ring". Assumption: the `P_EX100` files on the owner's PC (`local/re`) are the Steam game's.

### 2026-10-25: Stage 551 (testing branch: Hit reactions: KH2's air flinch turn)
- **KH2's air flinch turn.** The tester (m2, question 2: "(a)"; question 3: "(b)"). KH2's air flinch (`0x1403D3840`) sets the push direction, negated for a hit from the front, as the facing at once; its ground flinch (`0x1403D43B0`) doesn't turn (Checked (decomp)). KH1's air flinch `0x14029CD80` doesn't turn; its knock-back `0x14029CEC0` turns by the same rule through `0x140297E10 (actor, atan2(x, z))` and `+0x150` (Checked (decomp)). `hitreact::air_flinch_turn`, with Hit Reactions KH2 after KH1's air flinch (`0x4E` front / `0x4F` behind, air state 2): KH1's facing set on the push direction KH1 just gave his air velocity (`0x14029D800`), negated for `0x4E`; no drift → no turn. Steam only (`0x297E10`, as glide.rs). Flight and swimming keep KH1's numbers (already the case: KH2's push only on `0x48..0x4B`, the air hop / turn only in state 2; Checked (decomp) 0x14029D110 uses KH1's push with `0x4E`/`0x4F`). Open: the flight flinch still plays KH2's 14 / 15 and Medium Hits applies there. docs §15, §16.

### 2026-10-25: Stage 550 (testing branch: Hit reactions: KH2's hurt line with Battle Quotes KH2)
- **KH2's hurt line with Battle Quotes KH2.** The tester (soras-hit-reactions-native m2, question 1: "(b)"). KH2's reaction motions say their line from frame trigger 14 at frame 0: flinches 10 / 12 / 14 voice 29, alternates 11 / 13 and the air flinch from behind 15 voice 30, blows 16 / 18 voice 1 (`P_EX100.mset`); `00battle` VTBL for Sora 29 clip 6, 30 clip 5, 1 clip 7, priority 3 (Checked (files)). KH1's hit pass `0x14029BBD0` ends with the voice group `0x1402901F0 (actor, group, 0)`: 0 flinch, 1 knock-back, 0x14 low HP, 2 HP gone, after the reaction is queued (Checked (decomp)). `hitreact::hurt_line`, from the existing voice-group hook (`soundlog::voice_group`) and only during Sora's pass on KH1's states, says KH2's voice for the queued request (`0x49` / `0x4B` / `0x4F` → 30, other flinches 29, knock-backs 1) through KH1's voice path (`kh2sound::hurt_voice` → `kh1bank::voice`) and skips KH1's; low-HP and HP-gone lines stay KH1's; not loaded → KH1's. In KH2's borrowed moves the hook's replay of KH1's group 0 / 1 is skipped with KH2's quotes on (the reaction clip's trigger 14 already says it). Follows the Battle Quotes row whatever Hit Reactions says (asked). docs/kh1-hit-reactions-native.md §14.

### 2026-10-25: Stage 549 (testing branch: Glide: the glide's speed kept after letting go)
- **The glide's velocity goes on into the fall when Square is let go.** The tester (same thread: "momentum currently doesn't work with letting go mid glide I just drop"). The t530 traces: 30.1/frame at the release, 1.1 the next frame, then +0.8 a frame. KH2's end (`0x1403CBC30` with 0, KH2 Steam) changes only the arc and the launch flag `+0xEC`, never `+0x20`: the whole velocity carries on (Checked (decomp)). Our end ran in KH1's control step inside KH1's think `0x1402A3300` (KH1 Steam), which then (flag `0x400` set by the state change, state 2, after state 2's pre-step `0x1402AFC20` copied the stick into `+0x20`) sets `+0x3C0` = `+0x20` × `+0x2C`, KH1's own carry into a fall from the stick (Checked (decomp)). `glide::carry` (from `takeover::physics`, before KH1's physics step, as `shared::physics::ground_hop_carry` does for the same carry) puts the glide's velocity back on `+0x3C0` on the frame after a release in the control step, so KH1's fall step (physics KH1) or KH2's arc on it (physics KH2) go on at the glide's speed. Ends in KH1's physics step (ground, end height) aren't affected (`0x400` cleared at the next think's start by `0x1402A43A0`; the Aerial Dodge apex, also in the physics step, kept its speed in the same traces, Checked (trace)). Logged "KH2's glide velocity ... kept into the fall". Also docs §8.7: every glide value checked against the traces (fmab Glide LV1..MAX, Checked (files)). docs/kh1-glide-native.md §8.4, §8.7.

### 2026-10-25: Stage 548 (testing branch: Glide: KH2's left bank (207) plays)
- **KH2's left bank 207 plays on KH1's request 0x74.** The tester (kh2-glide-in-kh1s-glide-state, after t526..t531: "sora can't bank to the left but can to the right"). KH1's glide pre-step (Steam `0x1402B2880`) asks 0x73 for a turn past +30° and 0x74 past −30°; KH2's pick (`0x140423A70`) was right (206 when `(facing × stick).y > 0`, else 207; `0x140142120` is a × b with a = facing, Checked (decomp)). But the set Sora is bound to in a world (88 slots of its own) has 0x74's entry on slot 77, the right bank's, where the file has 78 (`xa_ex_0010.mset`: 0x73 `0x204D`, 0x74 `0x204E`, Checked (files)): the t530 log's "not in this set: 0x74", and the trace drawing KH1's slot 77 (KH2's 206) for every left bank (Checked (trace)). So 207 was never bound and both sides showed 206. `kh2slots::also_stands_for`: KH2's 207 stands in for 0x74 when the set's entry holds slot 78 or 77. Assumption: KH1's and KH2's worlds turn the same way (both y-down), which decides which side is 206. docs/kh1-glide-native.md §8.3.

### 2026-10-25: Stage 547 (testing branch: Quick Run: out of KH1's roll at KH2's frame)
- **KH1's own roll turns into Quick Run at KH2's roll's frame.** The tester (quick-run-native m3: "with kh1 ... waits until dodge roll finishes to go into quick run ... doesn't get the same distance ... make sure it behaves the same as kh2 dodge roll", "go ahead"). The trace: KH1's roll `0xDC` from frame 9384, Quick Run at 9414 (30 frames, KH1's roll motion's end); with Dodge Roll KH2 the log has it 10 frames in. `shared::control` let a held roll into `quickrun::start` only on a frame KH1's can-act test passed (KH1 holds its action flag to the roll motion's end); KH2's roll on KH1's roll already bypassed it. Now KH1's own roll is cut the same way at KH2's Quick Run check (motion 209's trigger 34, frame 10 at 60 Hz, counted on the same counter from the roll's first frame). The Quick Run itself was already identical both ways (log: speed 30, brake 0.94, time 14, stop after 13 frames at 19.5). Checked (trace / log / files: motion 209 length 52, trigger 34 at 10, root 220.8 by frame 10). Assumption: cutting KH1's roll at its 10th frame (its 12-frame no-damage timer flags cleared by `quickrun::start`, as for KH2's roll) needs nothing else of KH1's roll ended. docs/kh1-quick-run-native.md "Out of KH1's own roll at KH2's frame".

### 2026-10-25: Stage 546 (testing branch: Body guards: KH1 guard objects on every KH2 body guard window)
- **Every KH2 guard window on his body is a KH1 guard object on KH1's states.** The tester (thread "Body guards on KH2 moves", from aerial-dodge-native m2 question 7: "(a) go and investigate those moves and ensure that they have the proper frame data and timings in its own thread", "go ahead"). `takeover::kh2_hits` took body `Guard` windows only for the Aerial Dodge (t494); now every `Guard` window of the KH2 motion KH1 plays, as KH2's controller does (no filter on its guard boxes), so `combat::guard_step` puts all-round priority-100 guard objects on his body: Aerial Recovery 40 (ATKP 124, frames 0..16), Explosion 166 (124, 32..64), Magnet Burst 197 (406, 20..70), Aerial Dodge 202 (406, all) as before. Enemy attack objects meeting one are stopped (KH1's clash on their side), no reaction on Sora (refact self 2). Frame data checked against the files: guard, invulnerable (range 27: AR 0..16, Explosion 32..64; none on Magnet Burst / Flash Step / Vicinity Break) all match what the code reads; KH2's range test `0x14040E740` is start <= t < end, Checked (decomp), the same as `Window::contains`. ATKP 124 kind 3 (special guard), 406 kind 2, both refact self 2, reflect range 0, Checked (files). Assumption: kind 2 and 3 act the same against KH1's enemy attacks (no KH2 kind on them). Flash Step 169 / Vicinity Break 170 aren't baked (Keyblade guard windows rejected by `blade_swing` / `body_strike`), Checked (trace: t544 logs, 170 borrowed); KH2's controller plays them with all their guards. docs/kh1-clash.md.

### 2026-10-25: Stage 545 (testing branch: Aerial Dodge: KH1's own held jump on it at physics KH1)
- **The Aerial Dodge at physics KH1 is held as KH1's jump.** The tester (aerial-dodge-native m2: "currently sora in kh1 mode cancels his air dodge, that doesn't fit with how kh2 does air dodge in the base game", traces f80604 / f80807, Physics KH1, Jump KH1). Every dodge's arc was rewritten at air time 6 to KH1's release hop (5 frames, 6.06), the jump held or not: KH1's variable jump in Sora's button handler (Steam `0x1402A7B30`, from the control step `0x1402A6220`) cuts a rise in state 2 with `+0` flag `0x40000` on and `0x20000` (held) off from 6 frames in to 5 before the apex (`0x1402AF8F0 (actor, 5.0)`), and the dodge kept the flags of the jump before it, let go once (`0x6c003` -> `0x4c003` in the traces). `shared::physics::kh1_held_jump`, after KH1's control step in `takeover::shared_control`: a held hop started that step at physics KH1 (`kh2_hop_held`, the dodge) gets `0x60000` as KH1's own takeoff (`0x1402AFDA0`) sets it, so KH1's own variable jump runs on the dodge: held, the full rise; let go in that window, KH1's release hop and the fall swap at once (KH2's held-dodge rule `0x1403CB680`, same 6 / 5 frames, KH1's hop). Physics KH2 unchanged (`air_step` keeps KH1's cut off, runs KH2's release). Checked (decomp) KH1 Steam and KH2 Steam; Checked (trace). Also (docs only): the dodge's levels Checked (decomp + files) unchanged (LV1 145/18, LV2 155/24, LV3 200/30, MAX 300/36: `0x1403E92D0` record level - 1, `+4` / `+8`); the jump-press handler's glide table is ACTION_GLIDE's (`0x1405CDC38`, Checked (exe) RTTI, t496's assumption settled); a walk-off fall is ACTION_JUMP with the shared enter that gives the dodge (Assumption: no fall action exists in KH2's RTTI), as ours already does; 160 slot frames cover every rise. docs/kh1-aerial-dodge-native.md §7.

### 2026-10-24: Stage 544 (testing branch: Recoil: KH2 hits stopped by a guard on KH1's states recoil natively)
- **A guard that stops a KH2 hit on KH1's states meets it through KH1's own clash.** The tester (plan step 5). `strike_as(.., native)` from `takeover::kh2_hits`: `meet_shot` with the guard's attack object (new `Guard::obj`) against a swing of his KH1 row, so KH1's half `0x1402B4BC0` queues the kind 6 / 7 record on Sora and KH1's hit pass plays his recoil (KH1's or KH2's by the Recoil row); before, `register_wall_hit` went to a controller not driving him. Explosion / Vicinity Break / Magnet Burst are pierce armor in KH2 (Checked (files)), so at KH2 they meet no guard at all. Checked (decomp).

### 2026-10-24: Stage 543 (testing branch: Recoil: the guard repel's 80 frames)
- **KH2's guard repel gives 80 frames without damage (Recoil KH2).** The tester (plan step 4). `combomods::guard_repel` sets KH1's own no-damage timer to 80 (`+0xC` = 80.0, flag 0x80 on / 0x100000 off, as `0x140297F30`), KH2's `+0xD70` = 80 from `0x1403D61D0`. Checked (decomp); Assumption: KH2's step 1.0 a 60 Hz frame.

### 2026-10-24: Stage 542 (testing branch: Recoil: KH2's clash rules on our attacks)
- **KH2's attack kinds on our KH1 rows and KH2's hit shapes (Recoil KH2).** The tester (plan step 3; answers 2 "for now do (c)", 3 "(a)"). `kh1rows::rules` rewrites our rows' data live by the row: at KH2 pierce armor (ATKP kind 1: Guard Break 876, Counterguard 1581, Finishing Leap 854) gets KH1 clash type 6 (meets no attack or guard, hits bodies: KH1's pairing table `0x140528CB8` 1F 09 09 3F 29 18 00 00, Checked (exe); bodies kept from types 1 / 2 / 4 only in `0x1402B3BE0`, Checked (decomp)); refact self 2 (Horizontal Slash, Retaliating Slash) priority at least 0x62 (KH1's half `0x1402B4BC0` doesn't stop it; the enemy recoils by KH1's rule, kind 7 for a gap over 40, answer 3); at KH1 the templates' type and priority (Guard Break no longer passes guards at KH1). Data → object: `0x1402B2DC0` (`+0x38` bits 0..2 → flags 17..19, `+0x3C` → `+0x68`), `0x1402B3270` (Keyblade byte added, capped 99), Checked (decomp). `strike_as` at KH2: pierce meets only `SPECIAL_GUARDS` (empty: no KH1 enemy listed yet) and no attack; refact self 2 isn't repelled by a guard (its owner immune to the swing, KH2's `0x1403D2140`) and meets attacks at priority 99. KH2's ATKP rows Checked (files). Not KH2: a 0x62+ row passes through a guard to the body.

### 2026-10-24: Stage 541 (testing branch: Recoil: Config row, KH2's recoil on KH1's recoil requests)
- **Recoil row; KH2's 41 / 43 on KH1's recoil requests, KH2's push and hop, no protection.** The tester (answers 1 "(b)", 4 "should be part of a recoil switch in the config menu, if you switch to kh1 all moves including kh2 uses kh1's rules exactly and vice versa"). Config > Combat > Recoil (ini `recoil`, default KH1, `styles::Kind::Recoil`). At KH2 `kh2slots::RECOILS` plays KH2's 41 on KH1's 0x6E (slot 72) and 43 on 0x70 (slot 74); 0x6F (KH1's big recoil, slot 73) keeps KH1's animation. `recoil::after_pass` (from `hurt`, after KH1's hit pass `0x14029BBD0`): 0x6E's gap push (`0x1402C7300`) replaced by KH2's 50 back along minus his facing (`0x1403D4650` / `0x1403D4960`, on `+0x120` through `combomods::push_back`); 0x70's hop 20 + gap drift (`0x14029D510`) by KH2's 50 up and 50 × `hit_back_to_jump` back (`0x1403D4800`, `shared::physics::kh2_hop`); KH1's 8-frame no-damage timer (`0x140297F30`: `+0xC`, flag 0x80) put back as before the pass on all three (KH2 gives none). KH1's combo count less one stays (KH2's `+0xD74` the same). Checked (decomp) both games; Checked (files): request table `xa_ex_0010.mset`, 41 / 43 in `P_EX100.mset`. Assumption: one reaction per pass. Drive Forms / borrows keep KH2's controller recoil in both settings.

### 2026-10-24: Stage 540 (testing branch: Recoil: borrowed Keyblade swings meet enemy attacks)
- **The KH1 blade sweep goes on KH2's weapon 1, not the body.** The tester (kh2-recoil-and-parry-system m1, "go ahead"; plan step 1). `combat::Strikes::strike_as` took `h.weapon == 0` (KH2's body) as the right Keyblade, so a borrowed Keyblade swing never met an enemy's melee attack (only shots: the tester's logs) and body windows got the blade sweep. Now weapon 1; body windows and Valor's left blade keep KH2's shapes. Checked (code: `kh2_rig::attack_shapes`, `kh2_data::sora`); Checked (trace: the old logs' "a shot" meetings only). docs/kh1-kh2-recoil.md §6.1.

### 2026-10-24: Stage 539 (testing branch: Config > Combat: Criticals row)
- **Criticals row.** The tester (same thread, answer 2: "make it a config selection (criticals on or off for all moves that call it)"). Config > KH1 in KH2 > Combat > "Criticals: Off / On" (ini `criticals`, default Off, as KH2 has none). Off: in the `kh2damage` hook the attack object's critical chance (`+0x94`, compared with KH1's random roll in `0x1402BFA80`) is 0 for each of Sora's attacks for the call and put back after: no bonus, no record bit 0x20, no journal tally 8; Critical Plus does nothing. On: KH1's critical, KH2 hits included. Checked (decomp). Assumption: "all moves" means Sora's (enemies and partners untouched); the Combat list's eighth row fits as KH1's own 8-row list does. docs/kh1-kh2-attack-data.md §8.

### 2026-10-24: Stage 538 (testing branch: KH2's power in KH1's damage function)
- **KH2's power inside KH1's damage function.** The tester (kh2-attack-data-and-damage-on-kh1s-swings m1: "either a or b you pick which one the player is going to feel a difference in it feeling like kh2", "go ahead"); picked (a), KH1's curve × KH2's power, since KH1's enemy HP is tuned to its curve (§7). `kh2damage.rs` hooks KH1's damage `0x1402BFA80` (Steam, first 15 bytes `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18`): for an attack of Sora's with a known KH2 ATKP (a `land_hit` move, `combat::landing`; our rows, `kh1rows::kh2_atkp`; and now the native swings on KH1's own rows, data bone 0x30 with our slot playing, `kh1rows::native_atkp`) the curve pointer `0x142DE5DC0` points, for that call, at a copy of the curve × KH2's power / 100 × `damage_scale` (× the max-MP Magic stand-in, `combat::magic_factor`), rounded up as KH2; KH1's function then runs whole (critical, min / max, floor / cap, resistance, Aero, mode 2, Second Chance, at least 1). `scale_record` no longer rewrites `+8` or carries fractions while it's in (ini `kh2_damage = off`: the old way; Epic: the old way). KH1's MP charge per hit unchanged (answer 3a). Checked (decomp): the function and its one caller; Checked (exe): the prologue and its single load of the curve pointer at `0x1402BFBB4`. Assumption: (b) would have made late fights much longer (not measured). docs/kh1-kh2-attack-data.md §7.

### 2026-10-24: Stage 537 (testing branch: Combo at KH1: KH2's picks only as ability moves at KH1's count)
- **At Base / Aerial Base Combo KH1, KH2's combo length no longer decides anything.** The tester (same thread, m17: "changing the combat settings back to kh1 doesn't seem to change the row selection ... (double check)"). Checked in the code: plain rows, KH1's request table and the ledge (KH1's own row 2; KH2's drop test off) do switch; but `combomods::attack_row` still took KH2's pick by KH2's length, so with Combo Plus past KH1's length KH2's controller borrowed Sora for a plain KH2 swing, and an ability finisher (Explosion, Aerial Finish, Magnet Burst) could replace a KH1 swing before KH1's finisher. Now at KH1 a pick counts only when it is an ability move of the kind KH1's own count is at (swing before KH1's length, finisher at it); otherwise KH1's row and motion, logged ("not taken, KH1's own row"). KH2-only combo abilities (Finishing Plus, Negative Combo) still make it all KH2's (t488). Checked (code); not tested in game. docs/kh1-move-styles.md.

### 2026-10-24: Stage 536 (testing branch: Guard Repel row: KH1's own block at KH1)
- **Config > Movement > Guard Repel (KH1 | KH2).** The tester (KH2's combo row selection, m17: "(b)", the KH1 setting uses KH1's own reaction when the guard blocks). Ini `guard_repel = kh2 | kh1` (default kh2, today's behaviour). KH2: the repel as t489 (motion 42, pushed back 200, Counterguard). KH1: `combomods::kh1_block` gives Sora what KH1's clash `0x1402B4BC0` gives the owner of KH1's own guard (row 0x3D, data 0x4DA8: bone 0x30, clash type 2, priority byte 0, so the Keyblade's `+0x33` clamped 1..99 by `0x1402B3270`, under 0x62, not 100): a kind 6 reaction `0x14029D720` (request 0x6E, idle queued, 8-frame no-damage timer, combo count −1, pushed back by a negative gap) or kind 7 `0x14029D5F0` (0x6F, no push) when the attacker is more than 40 stronger, run at once with a local record (`+8` the gap), and the guard's result word (combat `+0x50`) marked 0x10 / 0x20, which KH1's Counterattack (`0x140294220`, command 99) reads with 0x6E playing. The attacker's priority from the pool attack our guard object met, or a hit record's data `+0x3C`. Steam-only constants, first bytes checked before each call. Checked (decomp), Checked (exe) for the prologues, Checked (files) for data 0x4DA8; Assumption: an enemy attack object keeps its data's priority; the attacker's own recoil stays KH1's clash on our priority-100 guard objects at both settings. docs/kh1-anim-slots.md §7.x.3.

### 2026-10-24: Stage 535 (testing branch: Physics KH2: KH2's hold with its arc clock running)
- **KH2's no-gravity hold runs the arc's clock (physics KH2).** The tester (m5 answer 1: "(b) unless you have kh1 physics on"). `shared::physics::air_step`: in a KH2 clip's range 3 / 24 on KH1's air state, physics KH2, the arc's time advances and only the move is 0, as KH2's ACTION_JUMP step `0x1403CB360` does (Checked (decomp) KH2 Steam), so he falls at once at the fall cap after the hold: casts, Aerial Dive (46..54), Finishing Leap. Physics KH1 unchanged; KH2's controller (Drive Forms, borrows) unchanged. docs/kh1-magic-native.md §4h.

### 2026-10-24: Stage 534 (testing branch: KH2 magic: KH2's air cast start (40 hop, momentum kept); no clip hover at physics KH1)
- **KH2's air cast start at physics KH2; no clip hover for casts at physics KH1.** The tester (m5 plan item 3; m4: "see how casting is done in the air physics wise to get the same for kh1"). `shared::physics::cast_stops` is now KH2's start (KH2 Steam `0x1403C5170` → slot 4 `0x140409B00` → `0x1403CBC30 (actor, 40)`, Checked (decomp)): a hop of sstm `magic_jump` 40 (Checked (files)) on KH2's arc from where he is, the sideways velocity kept (then `air_step`'s × 0.95 a tick drift in a move); KH1's command step's halving and 30 hop after the spell start are undone by `cast_after_command` (after KH1's command step in `combat::command`, end of `reflect::cast_native`). t487's stop is gone. At physics KH1 `air_step` no longer holds a cast slot in its clip's range 3 (`kh2slots::cast_playing`): KH1's own air cast has no hover (`0x1402AF6A0`, Checked (decomp)). Assumption: KH2's stick target is empty during a cast motion (drift × 0.95). docs/kh1-magic-native.md §4g.

### 2026-10-24: Stage 533 (testing branch: KH2 magic: Reflect on KH1's cast state)
- **Reflect on KH1's own cast state, no borrow.** The tester (same thread, m5 plan item 2, "go ahead"). `reflect::cast_native` (from `combat::command`, base Sora in KH1's own states 0 / 6 / 0x18 / 2, Steam) does what KH1's command step does for a spell (Steam `0x1402A7510` → pre-step `0x1402A38E0`, `0x1402A3600` / `0x1402A3DB0`, Checked (decomp)) with requests of ours (`kh1rows::OURS` "Reflect" / "Reflect (finisher)" / "Reflect (air)": KH2 motions 71 / 72 / 73, new `kh1rows::Kind::Cast` / `CastFinisher` / `AirCast` on KH1's Thunder cast slots 56 / 81, built as cast slots in `kh2slots` so KH2's combo window, cut and magic windows apply): the cast request with KH1's action flag, the idle (ground) or fall 6 blend 8 (air) queued, KH1's air hop `0x1402AF960 (btltbl +0xE8 = 30)` and `+0x3C0 × btltbl +0xF0 (0.5)` via `0x140386CD0` (btltbl pointer Steam `0x142D22D30`), the move direction normalised on the ground (`0x1402A93D0`); KH2's side: cost (`mpcost`), the combo step (`formmagic::note_step` / `take_step`, finisher cast 72 on the ground combo's last), voice, script. `takeover::native_spells` runs `reflect::native_step` on KH1's frames (slot motion and KH1's time; 4 frames' grace for KH1 to start the slot); `reflect::kh1_frame` no longer ends it in KH1's own states. Landing mid-cast is KH1's landing (its slot 11 drawn by the Jump row, the tester's m5 answer 2). Drive Forms, flight (state 8) and Epic keep the borrow. Checked (decomp); Assumption: KH1's Thunder request 0x38 / 0x85 entries are slots 56 / 81 (from t477's file notes). docs/kh1-magic-native.md §4f, docs/kh1-reflect-spell.md.

### 2026-10-24: Stage 532 (testing branch: KH2 magic: Cure KH1 is KH1's whole cast)
- **The Cure row at KH1 casts KH1's whole Cure.** The tester (kh2-magic-and-reflect-native m4: "cure kh1 would cure exactly like kh1 with its effects and cast while the cure switch for kh2 would switch"). With Magic / Aerial Magic KH2 and the Cure row KH1, `combat::command` no longer borrows Sora for Cure (KH1's command step `0x1402A7510` casts it as unmodded), `kh2slots::redirect` leaves KH1's Cure requests 0x39 / 0x86 on KH1's own slots 56 / 81, and `takeover::command_started` no longer puts KH1's Cure on KH2's Cure cast motion's clock (`formmagic::BASE_KH1_CAST` stays off for it): KH1's animation, script, effects, heal and cost. The Cure row at KH2 is unchanged. Checked (code); Checked (files) the request table (from t477's notes). docs/kh1-move-styles.md.

### 2026-10-24: Stage 531 (testing branch: Glide: Donald and Goofy glide after Sora with KH2's Glide)
- **Donald and Goofy glide after Sora with KH2's Glide.** The tester (question in m3: "(b)"). Their AI (`0x1401CF9E0`, members 1 / 2, from `0x1402A3300` → `0x1401CDBD0`) glides after his jump with KH1's Glide / Superglide only when Sora's ability bits (Steam `0x142D5EC10`, rebuilt by `0x1402A6380` on a menu close / map setup) have `0x80` / `0x100`. `glide::party`: at Glide KH2, base Sora with KH2's Glide and neither of KH1's, `0x80` set each Sora physics frame; taken off (only if ours, and he lacks KH1's Glide) when he can't glide with KH2's. Their glide stays KH1's. Sora's other reader of `0x80`, the fall state's jump press, is ours since t526 (no KH1 glide on his jump). `takeover::kh1_has_glide`. Checked (decomp) every read of the two bits. docs/kh1-glide-native.md §8.6.

### 2026-10-24: Stage 530 (testing branch: Glide: the borrow taken out, Square starts KH2's glide in KH1's state)
- **Square starts KH2's glide in KH1's glide state; the borrow is gone.** Same ask (plan step 5). `shared::control`: Square in the air at Glide KH2 calls `glide::start`; the jump in KH2's glide with the Aerial Dodge left ends it into the fall and starts the native Aerial Dodge (KH2's shared air slot `0x1403CB9F0`); in 0x15 at Glide KH2 KH1 sees neither the jump nor Square. Out: `shared::GLIDE`, `takeover::GLIDE_BORROW`, its launch hand-back and the dodge out of the borrow (t496, `DODGE_OUT_OF_GLIDE`), `combomods::native_dodge_built`. KH2's controller no longer glides base Sora in KH1's states (its Glide level kept aside, `KH2_GLIDE_KEPT`); Drive Forms keep theirs. Steam only. Checked (code / decomp). docs/kh1-glide-native.md §8.5.

### 2026-10-24: Stage 529 (testing branch: Glide: KH2's end and release in KH1's glide state)
- **KH2's end and release in KH1's glide state.** Same ask (plan step 4; walls question 2 "(b)", and KH2's glide has no wall rule). State 0x15's button check (Steam `0x1402B2BF0`) wrapped for Sora: Square let go ends the glide or hover at once (no 46-frame minimum); its transition (`0x1402B2780`): Neverland's flight first as KH1's (world flag `0x20`, `0x1402B1DA0`, state 8), then on the ground or under KH2's end height (growth `+0x38`, `0x1404235B0`) the end, KH1's wall end (moved under 70%) not run. `glide::end` (KH2's `0x1403CBC30`): KH1's fall from rest `0x1402AF8F0 (actor, 0)`, his velocity kept in `+0x3C0` (no x0.7, no hop of 8), KH1's fall request 6 with KH2's blend 16, KH2's launch flag cleared for the air time; on the ground KH1's fall state lands him the next frame. Checked (decomp); Assumption: KH2 ends on the release at once (not found in the glide action's own functions). docs/kh1-glide-native.md §8.4.

### 2026-10-24: Stage 528 (testing branch: Glide: KH2's turn, hover and motions in KH1's glide pre-step)
- **KH2's turn, hover and motions in KH1's glide pre-step.** Same ask (plan step 3; hover as a sub-mode of 0x15, the tester's question 1). State 0x15's pre-step (Steam `0x1402B2880`) wrapped for Sora: the hover picked with the stick neutral at the start (`0x1403A8369`), the stick takes it into the glide with the launch speed back (`0x140423840`), never back; the turn toward the stick at growth `+0x3C` a frame (`0x140423B90` / `0x140423BC0`) through KH1's facing set `0x140297E10`; KH2's motion pick (`0x140423A70`): 205, 206 / 207 past dot 0.98 by the side of the turn, 208 in the hover, blend 16, through KH1's request `0x14029F7C0`; KH1's water check `0x1402C12B0` still last. `kh2slots::GLIDES`: 205 / 206 / 207 on KH1's glide requests 0x71 (0x72) / 0x73 / 0x74 at Glide KH2 (slots 75..78 as templates); `kh1rows::OURS` "Glide hover" (208, request only, KH1's air swing template), its time put back to 0 at its end. Checked (decomp / files); Assumption: `0x140142120` is facing x stick (which side is 206), and KH2 loops 208. docs/kh1-glide-native.md §8.3.

### 2026-10-24: Stage 527 (testing branch: Glide: KH2's speed and fall in KH1's glide step)
- **KH2's glide speed and fall in KH1's glide step.** Same ask (plan step 2). State 0x15's step (Steam `0x1402B2A20`) wrapped for Sora: on KH2's physics tick (every two 60 Hz frames, dt 2, `0x1403B89A0`) the glide's velocity `facing x ((1 - a) * speed + a * |v|)` (`0x140423920`, growth `+0x30` / `+0x20`), the hover's x0.9 (`0x140423650`), the fall `(height - +0x28) * +0x24` capped at `+0x2C` from KH1's own `+0x3D4` (`0x1404237A0`); the displacement handed to KH1's mover (walls: he slides, KH2's glide has none), `+0x3C0` / `+0x3C4` KH2's velocity. KH1's +0.4 ramp and saw-tooth fall don't run for him. Checked (decomp); Assumption: KH2's collision doesn't cut `+0x20` on a head-on wall. docs/kh1-glide-native.md §8.2.

### 2026-10-24: Stage 526 (testing branch: Glide: KH2's glide start in KH1's glide state)
- **KH2's glide start, enter and exit in KH1's glide state 0x15 (not started yet).** The tester (kh2-glide-in-kh1s-glide-state: "(b)", "go ahead", plan step 1). New `glide.rs`: `glide::start` takes base Sora from KH1's jump / fall state into 0x15 with KH1's set-state (Steam `0x1402B1960`) above KH2's start height (growth `+0x34`), no KH1 timer read, his speed not halved, KH2's stored launch speed put back when KH2 has one (`takeover::launch_kept`, KH2's enter `0x1403CBD30`), else his speed as it is; KH1's request 0x71 with KH2's blend 16 (Steam `0x14029F7C0`). Wrappers on KH1's state table (Steam `0x142DB3840`, layout read from `0x1402B1B80`: `+8` enter, `+0x10` exit, `+0x18` pre-step, `+0x20` step, `+0x28` transition, `+0x30` button check, `+0x38` jump press), for Sora at Glide KH2 only (Donald and Goofy run KH1's): enter `0x1402B2D00` / exit `0x1402B29F0` keep `+0x1CC` bit 0 as KH1's but set the combat state's `+0x100` to 0 (no 46-frame minimum, no 24-frame wait: none in KH2's `ACTION_GLIDE` / `ACTION_GLIDEHOVER`); the fall state's jump press `0x1402AFD60` no longer starts KH1's Glide on him (Neverland's flight kept). `takeover::kh2_glide_growth`, `launch_kept`, `launch_used`. Not wired to a button until t530. Checked (decomp) both games. docs/kh1-glide-native.md §8.1.

### 2026-10-24: Stage 525 (testing branch: Guard: KH2's angle test for its guard windows)
- **The guard's coverage is KH2's ATKP angle test.** The tester (same thread, question 1: "go with whats most native and 1:1"). KH2 (`0x1403D1F40`, Checked (decomp)): a guard holds an attack when its ATKP reflect range is 0 or the attack's direction against his facing (turned by the reflect angle x 2) is below -cos(range); KH2 has no Guard Plus (the OG rewrite's `guard_all_round` is ours, never set). Sora's ATKPs (Checked (files)): 161 (guard press) 120, 162 (repel) 180, 406 / 124 / 455 0; angles 0. Before: 90 degrees each side for Keyblade guards. `hurt::guard_faces` (KH1's states and borrows; direction from the record's push `+0x10`) and the KH1 guard objects' data (`combat::guard_data`: `+0x64` cos(range), `+0x60` angle; KH1's directional test `0x1402B4640`, Checked (decomp)). Assumption: KH1's `+0x60` turn sign (unused). docs/kh1-roll-guard-native.md §7.

### 2026-10-24: Stage 524 (testing branch: Counterguard: KH1's Counterattack command during KH2's repel)
- **KH1's Counterattack command in Attack's place during KH2's guard repel.** The tester (same thread, question 2: "(a)"). KH1's swap `0x140294220` writes Attack's entry (`0x140528914`) each refresh, then calls the battle-menu fill `0x1402944C0` (Checked (decomp)); after that fill (`drive::fill`) `combomods::counter_entry` turns an Attack entry of 1 into 99 while our slot of KH2's 42 plays on the ground with Counterguard equipped (`counter_open`; no target needed, as KH2). Choosing it, `combat::command` makes the record KH1's Attack (type 1, target kept), so KH1's Attack step runs KH2's Counterguard through the repel's combo window (t489), not KH1's row 0x3E. Context commands (Talk) and the world entry 0x62 stay. Checked (decomp); Assumption: the fill runs every frame the menu is up. docs/kh1-roll-guard-native.md §6.

### 2026-10-24: Stage 523 (testing branch: Dodge Roll: KH2's roll on KH1's own roll)
- **KH2's dodge roll runs on KH1's own roll, no borrow.** The tester (dodge-roll-guard-and-counterguard-native m1 answer 3: "make dodge roll its own slot ... use kh1s as a base so its native"; "go ahead" after m2). With Dodge Roll KH2, Square on the ground with the stick pushed and KH2's Dodge Roll equipped (after the guard press, stick at rest, didn't take it: KH2's order `0x1403A81F0`) runs KH1's roll start (Steam `0x1402C9E60`) facing the stick, speed 0, KH1's 12-frame no-damage timer it sets (`0x140297F30 (actor, 12.0)`, flags `0x80 | 0x100000`) put back (`combomods::roll_press`). `kh2slots`' roll spec points request `0xDC` at our slot of KH2's 209 on template slot 117 at KH2: `0xDC` has KH1's travel flag (`0xA075`, Checked (files)), so the baked slot carries KH2's root travel; KH1's floor sounds and foot effects rescaled; KH1's "may be left" from KH2's fall-check event (34) in place of KH1's 30. `hurt` evades hits under the level's `dodge_roll_star_time` (10 / 16 / 24 / 32, `combomods::roll_invulnerable`, `takeover::kh2_dodge_roll_growth`). Square held through it: KH2's Quick Run at KH2's check frame. Borrow kept as fallback (forms, slot not built). Checked (decomp) `0x1402C9E60` / `0x140297F30`, Checked (files) request `0xDC` / slot 117; Assumption: KH1's ground step uses `+0x2C` under the action flag, nothing else reads `0x100000` as rolling; KH1's own untouchable flag `0x40` source still not found. docs/kh1-roll-guard-native.md §5.

### 2026-10-24: Stage 522 (testing branch: L2 Stick Menu row)
- **L2 Stick Menu row: L2 + the right stick is the d-pad, with either camera.** The tester (kh2-camera-fixes m14: "1. (a) but should be a toggle in the camera menu that kh1 camera and kh2 camera can use"). KH2's pad (Steam KH2 `0x14039c180`) turns the right stick past its dead zone (60 from 127.5) into raw bits 16..19; its static remap table (`0x1405c3420`) maps only the left stick's 20..23 onto the d-pad, so the right stick's d-pad use under L2 comes from the run-time PC table. Config > Camera: "L2 Stick Menu: Off / On" under L2 Camera Hold (ini `l2_stick_menu`, default off). With it on and L2 held, `camera::dpad_from_stick` (from `probe::pad`, on each pad record `0x140112C70` writes) adds the d-pad bits for the right stick past KH2's dead zone (diagonals give two) and centres that record's right stick, so KH1's pad step, command menu, repeat and Drive page flip take it as the d-pad and KH1's camera doesn't turn; KH2's camera drops its right stick and mouse under the same test (R3 stays L2 Camera Hold's). KH2's first person, the mouse-as-d-pad and the keyboard's camera keys aren't changed. Checked (decomp) KH2's direction bits and dead zone, Checked (exe) the static remap table; Assumption: KH2's run-time table pairs the right stick's bits with L2 onto the d-pad (the tester's description); KH1's repeat timing is used, not KH2's. docs/kh2-pad.md.

### 2026-10-24: Stage 521 (testing branch: Hit effects: KH2 spells call their own hit spark)
- **KH2's spells show their own hit spark, through KH1's own lookup.** The tester (answer 2: "(a) if a vanilla magic attack calls for its vanilla spark then let it, if its from kh2 let it call its own spark"; answer 1: Keyblade swings unchanged for now). `combat::SpellAttack::package` (formmagic's KH2 casts and Wisdom's shots, Reflect's pop) goes to `land_hit` with the ATKP row's effect_on_hit / random_effect: the attack object is made from a copy of its KH1 data (`hit_data`, one 4 KB page near the battle data via `kh1rows::near`, 32 slots) with `+0x28` = effect_on_hit - 1 (+ KH2's random step; -1 for 0: no spark, as KH2 `0x1403d1b40`), and its `+8` is the spell package's DPX (`kh2dpx::hit_list`, which writes the tagged entries' masks in KH1's numbering: KH2's kind 4 on KH1's 2, KH2's wall kind 2 dropped; entries whose effect runs programs KH1 lacks are unfindable, and if that is the one the hit would find KH1's spark plays instead). KH1's contact `0x1402b4060` -> `0x1402b4320` -> `0x1401dffe0` -> `0x1402ae650` then picks, places and turns KH2's element. KH1's own spells keep KH1's sparks (t345); with `kh2_effects` off or the package not built, KH1's spark as before. Checked (decomp) KH1's contact, lookup and record fill (kind 0x80 damage effect from the attacker's list, none in KH2 lists, as KH2 plays none there); Checked (files) the spell packages' hit elements. Assumption: a KH2 spell attack carries its spell file's PAX (Wisdom's shots the form archive), not traced to the line that sets KH2's attack `+0x28`. `docs/kh1-kh2-hit-effects.md` §6.3, §8.

### 2026-10-24: Stage 520 (testing branch: Hit effects: KH2's PAX lookup word kept, as KH1's DPX masks)
- **KH2's element lookup word kept and written as KH1's effect-list masks.** The tester (kh2-hit-effects-on-enemies, go-ahead on the plan). `kh2_formats::pax::PaxElement::tag` keeps the `+8` word KH2's lookups search (`0x1402c6730`, Steam KH2) instead of dropping it; `kh1dpx::Variant::tag` writes its low half into the built DPX entry's `+0x10` (KH1's kind mask) and its high half into `+0x14` (group mask), the masks KH1's own lookup `0x1401dffe0` (Steam) searches; `kh2dpx` passes each element's word through and logs it. Nothing plays differently yet (no KH1 lookup reads our DPXs). Checked (decomp) both lookups; Checked (files): every KH2 DPX entry checked has 0 in those words, and the spells' hit elements carry kind 1 with groups 1 / 2 (`docs/kh1-kh2-hit-effects.md` §6.1, §6.2). Test `element_tags_become_kh1_masks`.

### 2026-10-24: Stage 519 (testing branch: Sleeves thread: the coat's hem hangs from the hips)
- **The coat's top stays on the chest, its hem lies on the hips.** The tester (missing-sleeves-animation, after t500: "the coat is still sticking out on kh2 animations" / "don't worry about the kh2 model for now"). `examples/coat_check.rs` now measures how far the coat's lower joints stand off the hips' up axis against the bind (and the same over KH1's own motions, `kh1 <mset>`): carried whole by the chest (t491) they stood more than 2 out in 42% of KH2's frames (KH1's own coat: 15%), the back 6.0-6.6 out behind the hips and the +x front 7.3-8.2 into them through the whole idle, walk and run (KH2's chest leans forward of its hips). `retarget::coat_low` (live and baked): each chain's root and root + 1 stay rigid on the chest as KH2's jacket; root + 2 goes halfway, the tip all the way to where KH1's hips (3) carry it in the bind, segments re-aimed, lengths kept (`COAT_LOW` 0.5 / 1.0). After: out >2 0% (max 1.5), into the hips >2 10.1% (KH1's own 12.7%), nearer a thigh than the bind 14% (was 22%); front tips inside the shorts 0.57% of joint-frames (was 0.17%; kicks, the run at most 2.3), no pop above the knees' in any motion. Checked (files): 83 motions of `P_EX100.mset`, every half frame. Assumptions: the shorts' radius from KH1's panel bones; KH1's own motions in plain FK (IK helpers not evaluated). Seven other rules measured (table). Test `local_own` (coat top rigid, hem never over 2.5 off the hips, lengths kept). docs/kh1-secondary-motion.md "The coat's lower half".

### 2026-10-24: Stage 518 (testing branch: Reflect: the barrier's blocking shapes at KH1 Sora's size)
- **Reflect's blocking area at KH1 Sora's size.** The tester (same thread, m18 answer 2: "if sora is smaller than kh2 sora then yes scale it down"). KH1 Sora is 0.89 of KH2 Sora (hips 78.9 against 88.5 in the two bind poses, `Retarget::hip_ratio`, Checked (files)); `reflect::kh1_sized_barrier` scales the barrier's attack shapes (ATKP 573) about Sora's place by `pose::kh1_size` (centre offsets, radius, height) before `Strikes::barrier_meet` meets enemy attack objects with them, as t483 scaled its effects. `hurt`'s stop of hits while the barrier stands tests no shape (unchanged); the counter / pop shapes stay KH2's. docs/kh2-effects.md §18.22.

### 2026-10-24: Stage 517 (testing branch: KH2 effects: KH2's object frame is the actor's rows (half turn on KH1's bound frame))
- **Reflect's ground barrier on his raised hands.** The tester (same thread, m18: "reflect on the ground is the one currently not placed correctly"). KH1's actor matrix (`0x1402A0A40`): `+0xD0` = facing × (0, 1, 0), `+0xF0` = 0 − facing (`+0x30`), `+0xE0` = `+0xF0` × `+0xD0`; the skeleton pass `0x140391C70` (from `0x14029F0A0`, given `actor + 0xD0`) draws the model with rows 1 and 2 negated, so the rows are the model's (x, down, back), KH2's object frame exactly; §18.21 had `+0xF0` as ahead (Checked (decomp)). KH1's 0x8000 frame read straight times its apply's `P` is the model's (−x, down, forward): KH2's turned half round about y. The t515 log's origin line for Reflect 3's element 0, (19.9, −76.8, 16.4) along the rows against KH2's (−19.6, −75.7, −16.2), agrees (Checked (trace)); the files put those pieces by his hands in motion 71 (Checked (files)). `kh2dpx`: bound copies on 0x8000 / 0x8005 take `H · t`, `H · R` (`HALF_TURN_Y`); position-only offsets go through `P · H` (`KH2_IN_KH1_OBJECT`). Also changes every object-root piece with an x / z offset or a turn (Explosion, Guard Break, Finishing Leap, Aerial Finish, forms' root effects, 0x4001 / 0x4009). Assumption (open): 0x8005 reads 0x8000's frame. docs/kh2-effects.md §18.23.

### 2026-10-24: Stage 516 (testing branch: KH2 magic: the cast motion for the spell script's first frame)
- **KH2's spells come out on the first press again.** The tester (kh2-effects-in-kh1s-own-effect-system, m18: "kh2s magic doesn't get casted the first time you input said magic"). Since t477 KH2's cast runs in KH1's own cast state: `formmagic::kh1_spell_start` (KH1's spell start `0x14028F140`) starts KH2's script, but KH1 only queues the cast's request there and plays it from its next animation step, so the script's first frame saw Sora still in his idle (or our slot at a time below 0). KH2's spell scripts end a cast at once when the caster isn't in the cast motion on their first run: `FIRE_3` / `BLIZZARD_3` / `THUNDER_3` run with `kh2-magic` and a first-frame motion other than 56 end that frame with no release and no attack, with 56 they release and attack (Checked (files)); the t515 log and traces show every combo's step 1 with no hit and steps 2 on hitting 24..26 frames in, the motion already playing (Checked (trace)). `formmagic::shots_step` now gives the scripts' world (`KhWorld`) Sora in the cast motion until the cast is seen playing (`casting`'s `SEEN`); KH1 is untouched. docs/kh1-magic-native.md §4.

### 2026-10-24: Stage 515 (testing branch: KH2 sounds: kh2_sound_engine = mod keeps KH1's engine out)
- **`kh2_sound_engine = mod` keeps every KH2 bank out of KH1's engine.** Found with the startup crash (same thread): `kh1bank::frame_end` registered the banks and the voice bank whatever the ini said; only the plays followed it, so the switch-back wasn't one. It now returns at once unless the engine setting is kh1. Checked (code).

### 2026-10-24: Stage 514 (testing branch: KH2 sounds: the voice bank's registration gets its name (startup crash))
- **Startup crash: KH1's voice-bank registration was missing its name.** The tester (kh2-sounds-through-kh1s-sound-system: "crash when starting the game ... if you find the reason I give you permission to fix it. go ahead"). t513 called KH1's `0x1400E0110` with three arguments; it takes four, r9 the entry's name, used as the format of `sprintf(entry + 0x30, name)` (`0x140051260`) before the entry is linked (Checked (exe): `0x1400E0132`, `0x1400E0279..0x1400E0280`; the decomp shows three). KH1 formatted whatever r9 held as soon as its sound system was up. KH1's own voice path `0x1400DFE60` takes the name in r9 too (Checked (exe)). Now `kh2_tt0_sora.win32.scd` is passed (≤ 0x20 bytes, the 0x50-byte entry), empty / oversized banks skipped, a busy lock retried next frame. Assumption: this was the tester's crash (no log sent).

### 2026-10-24: Stage 513 (testing branch: KH2 sounds: battle quotes through KH1's voice path)
- **KH2's battle quotes through KH1's voice path.** The tester (question 2: "what ever is closet to how kh1s systems handle it"). KH2's voice bank goes into KH1's voice list (`0x1400E0110`, a free bank number from 127 down); a quote is picked by KH2's rule (`0x1403CDB60`) and asked of KH1's `0x140290930 (Sora, clip, VTBL priority)` with `0x142D37270[member]` swapped for the call: KH1's priority, channel `0x142D371D0`, mouth motion and voice bus. Old player the fallback. Checked (decomp); Assumption: KH1's voice create plays KH2's Vorbis clips.

### 2026-10-24: Stage 512 (testing branch: KH2 sounds: buses 4..8 on KH1's effects bus)
- **KH2's buses 4..8 on KH1's effects bus 2.** The tester (plan step 3). KH2's effects slider (`0x140138290`) drives buses 2 and 4..8 alike, KH1's (`0x1400DDD10`) 1 / 2 / 3 (5 / 6 to a scale), never 4, 7 or 8: cut banks on 4..8 (the Keyblade bank se500's 8) go on bus 2. Checked (decomp).

### 2026-10-24: Stage 511 (testing branch: KH2 sounds: effects through KH1's effect queue at Sora)
- **KH2's effects through KH1's effect queue at Sora.** The tester (plan step 2; question 3 "same as 2"). Queued with KH1's `0x140179AC0 (no, id, Sora+0x10, 0)`; KH1's queue step `0x1401791E0` creates, pans and plays them; our slot is stopped with KH1's `0x140179D90` once its length has played in game time (`0x1401789F0`), as the step never frees slots (Checked (exe)). The old player is the fallback (not Steam, bank not loaded, queue full). Ini `kh2_sound_engine = kh1 | mod` (default kh1). Checked (decomp); Assumption: the engine plays the cut banks.

### 2026-10-24: Stage 510 (testing branch: KH2 sounds: banks registered in KH1's engine)
- **KH2's sounds registered as KH1 banks (nothing plays through them yet).** The tester ("go ahead", plan step 1). `kh1bank.rs`: each sound a `se9NNNNN.win32.scd` bank registered with KH1's `0x1400E0F00` on a copy from KH1's `_aligned_malloc` (IAT `0x1403B0778`; KH1 frees banks with `_aligned_free`), put back after KH1's bank list `0x1404D65D8` changes (`0x1400D2C70(1)` → `0x1400DBAC0` releases all, from the map tear-down steps), only while holding KH1's adding semaphore (`[0x1421AAFD8]+0x18`) and with more than 128 slots left (`0x14004F570`); logged ("KH1 banks:"). Checked (decomp); "room unload" an Assumption.

### 2026-10-24: Stage 509 (testing branch: KH2 sounds: one-sound banks in KH1's file form)
- **One KH2 sound cut into a one-sound bank, KH1's own file form.** The tester (kh2-sounds-through-kh1s-sound-system m1: "1. a"). `kh2_formats::scd_cut`: a one-sound SEDBSSCF bank (record, tracks, layout, attributes, audio; renumbered, laid out as KH1's own); KH1 reads SCD versions 1..4 (`0x140078D40` / `0x140078DA0`, Checked (decomp)). Cutting sound 0 of KH1's own `se021010` / `se017113` / `se021073` gives them back byte for byte; every playable KH2 id decodes the same except 35 se000 ids our old reader played from the wrong audio (index − 152: Magnet Burst 169, Thunder 126..128 / 148, Reflect 138 / 139 / 176), Checked (files). docs/kh1-sound-native.md.

### 2026-10-24: Stage 508 (testing branch: Hit reactions: Medium Hits row)
- **Medium Hits row.** The tester (answer 2: "how sora receives knock back should also be a config"; answer 3). "Medium Hits: Blow / Flinch" (ini `medium_hits`, default Blow): at Flinch, KH1's kind 4 (20..29% of HP, `0x1402BF290`) becomes kind 3 before the pass on KH1's states with Hit Reactions KH2; in base Sora's borrows the controller flinches. Re-hit: KH2's rule (`0x1403CF600` / `0x1403CF230` / `0x1403D4880`) and KH1's (`0x140297F30` / `0x140297FE0`) both give 30 frames after the reaction ends; only KH2's same-group exception differs, which KH1's attacks can't trigger, so KH1's timer is kept. Checked (decomp).

### 2026-10-24: Stage 507 (testing branch: Hit reactions: KH2's push on KH1's push)
- **KH2's push on KH1's push (Knock-back Numbers row).** The tester (answer 1: "make it a switch in the config ... your judgement"). `hitreact.rs`: after the pass, KH1's flinch push speed (`0x1402C7300`) is KH2's 0.1 × distance, and each frame KH1's push step `0x1402C7070` moves him by KH2's speed with KH2's ×0.9 decay (`0x1403D4960` / `0x1403D4570`); the air flinch gets KH2's hop 50 / drift × 0.048 (`0x1403D4800`) through `kh2_hop`. Config "Knock-back Numbers: KH1 / Scaled / KH2" (ini `knockback_numbers`, default KH2, as `0x1403CEAB0` falls back to sstm's 200 for attacks with no numbers). btltbl block Steam `0x142D22D30`. New three-value row type `config::Item::Pick` (Assumption: its panel layout). Checked (decomp).

### 2026-10-24: Stage 506 (testing branch: Hit reactions: KH2's flinches on KH1's reaction requests)
- **KH2's flinches on KH1's reaction requests (Hit Reactions row).** The tester (soras-hit-reactions-native m1: "go ahead"). Config > Combat > Hit Reactions (ini `hit_reactions`, default KH1). At KH2, `kh2slots::REACTIONS` plays KH2's 10 / 11 / 12 / 13 on KH1's ground flinch requests 0x48..0x4B (slots 61..64) and 14 / 15 on the air flinches 0x4E / 0x4F (67 / 68); KH1's reaction functions and the hit pass `0x14029BBD0` still run the timer, push, voice, HUD and MP; KH2's free state from 50 is KH1's "may be left"; KH2's clip voices left out. Checked (decomp): KH1's 0x48 is front by KH2's own test (`0x1403D2FE0`); Checked (files): both msets' lengths. Knock-backs wait for KH2's blow state. docs/kh1-hit-reactions-native.md.

### 2026-10-24: Stage 505 (testing branch: Quick Run: KH2's start effect above LV1)
- **Quick Run's start effect above LV1.** Same ask (the research's open point). KH2's start calls `0x1403B6670(actor, level + 0x6C)`, a PAX element by id (`0x1402C6B30`): elements 110 / 111 / 112 of `P_EX100.a.us` for LV2 / LV3 / MAX, started through `kh2fx::event`; 203 joins `kh2fx::DEFAULT_MOTIONS`. Checked (decomp / files).

### 2026-10-24: Stage 504 (testing branch: Quick Run: invulnerable frames 1:1)
- **Quick Run's invulnerable frames 1:1.** The tester (question 1: "make behave exactly 1:1 with kh2 so if its (a) then do it"). Stage 104's 6-frame minimum (`QUICK_RUN_STAR_MIN`) is gone: KH2's fmab 0 / 8 / 12 / 16 by level, and `hurt` stops hits during the native run's frames (`quickrun::invulnerable`). Checked (files); Assumption: KH2's counter `+0xD70` counts 60 Hz frames.

### 2026-10-24: Stage 503 (testing branch: Quick Run: off a ledge)
- **Quick Run off a ledge.** Same ask. KH2's slot 4 (`0x140409B00` → `0x1403CBC30(actor, 0)`): a fall from rest keeping the velocity, the Aerial Dodge left (AIRSLIDE isn't airborne, its `+0xC` bit 0 clear in the exe's action table `0x140750D30`). Ours: KH1's fall from rest as `0x1402AF8F0` writes it, state 2 set by our transition (so the physics step's 8-unit walk-off cap doesn't run), the slide's velocity in `+0x3C0`, fall request 6 blend 8. Checked (decomp / exe); Assumption: the fall motion (KH2's fall action asks none).

### 2026-10-24: Stage 502 (testing branch: Quick Run: the start points start the state)
- **Quick Run's start points start KH1's Quick Run state, no borrow.** Same ask. `shared::control`: Square held through KH1's roll (its roll flags `0x80 | 0x100000` cleared as KH1's think does, `0x140297FE0`), Square while moving without KH1's Dodge Roll, and KH2's action button picking Quick Run at Dodge Roll KH2 (KH2's order `0x1403A75A0`: guard at rest, roll, Quick Run) call `quickrun::start`. The borrow stays the fallback (forms, Epic, requests not built). Checked (decomp).

### 2026-10-24: Stage 501 (testing branch: Quick Run as KH1 movement state 0x2A)
- **KH2's Quick Run as KH1 movement state 0x2A.** The tester (quick-run-native: "do your first purposed route ... make behave exactly 1:1 with kh2"). `quickrun.rs`: state 0x2A in KH1's state table (Steam `0x142DB3840`, flags 0x08, our step and transition): KH2's start `0x1403A9830` (fmab speed, brake, time, stop brake, invulnerable time; `takeover::kh2_quick_run_growth`), hold timer and stop `0x140423500` (counts while Square held, 0 on release; stop brake and motion 204), the velocity step on KH2's 2-frame physics tick (`0x1403B89A0`), the move through KH1's ground step (Steam `0x1402B1850`), `+0x3C0` the velocity; motions 203 / 204 as request-only `kh1rows` rows; KH1's landing at 204's end. Checked (decomp) KH2 and KH1, Checked (files) 203 / 204; Assumption: 204's end into KH2's landing. docs/kh1-quick-run-native.md §6.

### 2026-10-24: Stage 500 (testing branch: Sleeves thread: coat research tool)
- **Coat research tool.** Same ask. `kh1-data/examples/coat_check.rs` measures the chest-rigid coat (t491) against eleven candidate rules over 83 motions: none removes its brief clips at the top of the thighs (0.15% of half frames, side kicks / splits / spins) without one-frame pops or more clipping, so the coat stays as t491. No behaviour change. Assumption: the shorts' radius from KH1's panel bones. docs/kh1-secondary-motion.md 'The coat against the thighs'.

### 2026-10-24: Stage 499 (testing branch: Sleeves thread: thighs and shins roll as KH2's skinned leg bones)
- **Thighs and shins roll as KH2's skinned leg bones.** The tester (missing-sleeves-animation, after t491: "keep researching until we have a good looking animations that emulate kh2s animations as close as possible while still making it look natural to kh1's model"). KH2 draws its thighs on the shorts' leg bones 24 / 53 and its shins on the knees 12 / 41 (Checked (files): `P_EX100.mdlx`); the retarget rolled them by the knee's direction, up to 109° / 62° (thighs) and 44° / 69° (shins) off those bones (motions 8, 7, 169, 153, 40, 183, 204, 42, 171). `retarget::Pole::Knee`'s `skin`: each limb aimed along KH2's joints and rolled as its skinned bone (turn from bind on the part's bind z), the knee's direction only where that z nears the limb's line. After: at most 0.2° (Checked (files), 83 motions every half frame, `examples/leg_roll.rs`); largest half-frame turn = KH2's own. Live and baked; `inverse` (KH2 model, off) keeps the knee rule. docs/kh1-secondary-motion.md.

### 2026-10-24: Stage 498 (testing branch: L2 Camera Hold row)
- **L2 Camera Hold row.** The tester (kh2-camera-fixes m13: "2. (a) but make it a toggle in the cameras config menu"). Config > Camera: "L2 Camera Hold: Off / On" (ini `l2_camera_hold`, default off): with it On and L2 held, KH2's follow and lock-on cameras take no right-stick turn or zoom, no mouse motion and no R3 reset, as KH2's L2. L2 is read as `camera::r2_held` reads R2: KH1's raw pad word `0x14298C9B4` bit `0x100` (Steam). First person, KH1's soft-target drop (`0x1402B11B0`) and Frame Advance (R2 only) untouched; no stick-as-d-pad yet (asked). Checked (decomp / exe) KH2's camera bit tests `0x140178AB0`..`0x140178B80`, masks `0x14071AA98` / `0x14071AA88`; Assumption: KH2 ties L2 to them in its run-time remap table `0x140BF3330`. docs/kh2-pad.md.

### 2026-10-24: Stage 497 (testing branch: Lock-on side kick, KH2's rule)
- **Lock-on side kick, KH2's rule.** The tester (kh2-camera-fixes m13: "1. (a)"). KH2's lock-on camera (Steam KH2 `0x140176EC0`, screen test `0x140176710`) tests the target first, Sora (100 above his feet) only if the target is on screen: off by the left edge (bit 4) the goal yaw is base + 20°, else off by the right (bit 8) base − 20°, top / bottom only none; left wins. Ours took the sign from the way the camera was turning. `kh2_camera::lock_side` is KH2's rule; the crate's frame is KH1's with y flipped, so its screen sides are mirrored and KH2's bit 4 is the crate's `screen::RIGHT` (`kh1-render.md` §1.3). Checked (decomp) the rule and base yaw `0x140176A70`; Assumption: KH2's screen handedness is KH1's (a left / right trace settles it). docs/camera-model.md.

### 2026-10-24: Stage 496 (testing branch: Aerial Dodge out of KH2's glide, native)
- **The Aerial Dodge out of KH2's glide goes native.** The tester (aerial-dodge-native m1, question 2: "(b) full native, glide will get converted"). In KH2's glide's borrow, a jump press with the dodge left ends the borrow before KH2's step (`shared_give_back` into KH1's fall state, the glide's launch kept) and starts `combomods::aerial_dodge_with` on KH1's air state (numbers from KH2's controller while held). Only with our dodge request built (`native_dodge_built`), else the controller's dodge as before. KH2's `0x1403CB9F0` in the air actions' tables Checked (decomp); which is the glide's: Assumption. Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 495 (testing branch: Aerial Dodge: the early release)
- **The Aerial Dodge's early release.** The tester (aerial-dodge-native m1, plan item 4). KH2's dodge is holdable: bit `0x10000` → the shared enter's held jump `0x20000` (`0x1403CBD30`), release marked by `0x1403D6FC0`, cut by `0x1403CB680` (fall swap `0x1403CB610`, then the release hop), Checked (decomp). The native dodge's hop is held (`shared::physics::kh2_hop_held`), `air_step`'s existing early release cuts it at physics KH2 and asks KH1's fall at once (`combomods::dodge_fall`, shared with the apex). Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 494 (testing branch: Aerial Dodge: its guard as KH1 guard objects)
- **The Aerial Dodge's guard (ATKP 406) as KH1 guard objects on the native slot.** The tester (aerial-dodge-native m1, plan item 3). `takeover::kh2_hits` takes 202's body guard window too (others only on the Keyblade), so `combat::guard_step` makes the all-round KH1 guard object (priority 100) KH2's controller made for its dodge: enemy attack objects meeting it are stopped, no reaction (`refact_self` 2). Hits his hit pass finds are still `hurt`'s. Checked (files): 202's window and ATKP 406. Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 493 (testing branch: Aerial Dodge: the fall at KH2's apex)
- **The Aerial Dodge's fall at KH2's apex.** The tester (aerial-dodge-native m1: question 1, my pick; plan item 2). `combomods::dodge_apex` (before Sora's KH1 physics frame): with our dodge slot playing in KH1's air state, 5 frames before the apex by KH1's arc fields (`+0x3B0` + 5 >= `+0x3B8`) and nothing pending (`+0x178`), KH1's fall request 6 with blend 8, action flag off. KH2's rule `0x1403CB680` / `0x1403CB610` (motion 4, blend 8.0) Checked (decomp); KH1's apex swap `0x1402AF630` only for its own requests, Checked (decomp). Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 492 (testing branch: Aerial Dodge on KH1's air state)
- **KH2's Aerial Dodge on KH1's air state, no borrow.** The tester (aerial-dodge-native m1: "go ahead", plan item 1). The fresh jump in the air with Aerial Dodge: `combomods::aerial_dodge` asks our request of KH2's motion 202 (`kh1rows` "Aerial Dodge", request only, slot on KH1's air swing 102 with its sounds left out, 160 frames with KH2's loop baked in), state 1 put in state 2 as KH1's launch (Steam `0x1402B1960`) does, facing the stick, KH2's hop with the `fmab` height / speed of the level (KH2 `0x1403CB9F0` / `0x1403CBB00`), launch kept for KH2's glide; KH1's air step steers it as a jump. The borrow stays as the fallback (forms, slot not built). Checked (decomp) KH2's handler and KH1's loop at a slot's end; Assumption: 160 frames covers every rise. Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 491 (testing branch: Sleeves thread: the coat moves with the chest as KH2's jacket)
- The tester (same answer). KH2 draws its jacket on torso bone 67 only, which turns as the chest part; its coat ring (129-135 / 221-227, 70) carries no mesh (Checked (files)). KH1's coat (72-75, 248-279) is carried rigidly by KH1's chest 78 as in the bind (`retarget::ATTACHED`), live and baked. Out: the pelvis turn, the front panels' thigh share and `keep_clear` on its nine chains (ours, s284-t489). Measured over 2954 frames: rigid to 0.08°. KH1's coat is longer than KH2's jacket, so its front tips reach 2.9-3.5 from a thigh's axis in kicks (Checked (files)); open question in the thread. Test `local_own`. docs/kh1-secondary-motion.md "The coat".

### 2026-10-24: Stage 490 (testing branch: Sleeves thread: shorts hang on KH1's thigh as KH2's)
- The tester (Missing sleeves animation, after m11): "make everything 1:1 with how its animated in kh2, if there's no equivalences like the ones we talked about then don't add them". KH2 draws its shorts on its pelvis 2 and on 24 / 53, which the thighs carry; its trouser bones (20, 22, 27, 29, 32, 34 and the other leg's) carry no mesh. 2 turns as the pelvis part, and 24 / 53 as the thigh part in the median frame, over all 83 motions (Checked (files): `P_EX100.mdlx` / `.mset`). `retarget::KH1_OWN` is empty: the shorts' panels (26-41 / 52-67) no longer take KH1's own animation (t387) and keep their bind relation to KH1's thigh, live and baked; the `set_own` machinery is kept with no bones. KH2's 9-30° key of 23 / 24 off the thigh joint (203-208, 42, 171) isn't carried. Test `local_own`. docs/kh1-secondary-motion.md "1:1 with what KH2's model shows".

### 2026-10-24: Stage 489 (testing branch: KH2's guard, its repel and Counterguard on KH1's states)
- The tester (m15 3 (a); 4 "(a) but read the mset"). Square standing with Guard (Dodge Roll KH2) is our KH1 request 0x208 for motion 173 (no borrow; KH1's action flag, idle queued, lock faced; `combomods::guard_press`); its Keyblade guard 161 (6..37, refact 1) stops hits from in front through `hurt` (`kh2slots::keyblade_guard`) and KH1 guard objects (`combat::guard_step`); the repel is request 0x209 / motion 42 faced to the attacker, pushed back 200 through `+0x120`; Counterguard is request 0x20A, row 39 / motion 171, ability 96, taken by KH1's Attack step in a combo window opened during the repel. Range id 2 (KH2 `0x14040D810` bit 0x10, Checked (decomp)) is `WindowKind::Free`, KH1's "may be left" on our requests. Guard Break: W_EX010 165's hit 876 is kind 1 (pierce armor) plus its own guard 162 at 5..42 (Checked (files)); KH1's priority rule kept (Assumption: kind 1 is KH2's guard break). Assumption: KH1's ground step moves him by `+0x2C` under the action flag. docs/kh1-anim-slots.md §7, docs/kh1-clash.md.

### 2026-10-24: Stage 488 (testing branch: KH2-only combo abilities make the combo KH2's)
- The tester (KH2's combo row selection, m15): "go with the recommended choice you had then" ((b): KH2's picker and rules) and "yes only the ones kh2 doesn't have". With Finishing Plus, Negative Combo or Berserk Charge (107) equipped, `styles::kh2(GroundCombo | AirCombo)` reads KH2 whatever the setting; `styles::setting` keeps the Config row showing the player's choice; `menu` feeds `styles::note_abilities` on each ability change (logged). Combo Plus / Air Combo Plus / Combo Master (KH1's own) switch nothing. Berserk Charge isn't in the ability list yet. docs/kh1-move-styles.md.

### 2026-10-24: Stage 487 (testing branch: KH2's stop at an air cast)
- The tester (m3 answer 2 (a), "with the exception to drive forms for now"). `shared::physics::cast_stops` (from `kh1_spell_start`, air state 2, base Sora, physics KH2): sideways speed 0 and KH1's arc reset to a fall from rest, then KH2's gravity and the cast's no-gravity hold (range 3). KH1's own air cast keeps 0.7 of the speed and the rise (`0x1402A3600`, Checked (decomp)). Assumption: KH1 doesn't move him on the ground while its action flag is set. docs §4d.

### 2026-10-24: Stage 486 (testing branch: KH2's casts as a step of KH1's combo)
- The tester (m3 answer 1 (a)). Cast slots get KH1's combo window (event type 5) from KH2's range 14 to the end (`kh2slots::cast_follow`; Fire 56 from 30, Blizzard 59 from 24, Thunder 62 / Cure 65 from 26, air 58/61/64/67 from 36/44/38/38, finishers 57/60/63 from 36/48/42; Checked (files) `P_EX100.mset`), finisher casts none; a fresh jump or Square there clears the action flag (`formmagic::cast_cut`); KH1's count `+0x41` and `+0x50` set at the native start (`formmagic::take_step`; KH1's Attack step `0x1402A7510`, `+0x50` cleared by `0x1402B3270`, Checked (decomp)); spells only from range 35 (`cast_magic_window`). Open: Finishing Plus after a finisher cast. docs/kh1-magic-native.md §4c.

### 2026-10-24: Stage 485 (testing branch: Config > Magic: Cure row)
- The tester (m3 answer 5): "make a menu in the magic config to change what gets casted kh1 cure or kh2". `styles::Kind::Cure`, ini `cure_style` (default kh2, as before), a Cure row in Config > KH1 in KH2 > Combat > Magic. KH1 = KH1's own Cure drawn on KH2's Cure cast motion and run on its clock, as Gravity / Stop / Aero at Magic KH2 (`formmagic::kh2_cure` / `kh2_spell`, `native()`, `spell_chosen`, `takeover::command_started`). Only with Magic KH2, not in Drive Forms. Trace column `cfg_cure`. docs/kh1-move-styles.md.

### 2026-10-24: Stage 484 (testing branch: KH2's MP costs on KH1's MP)
- The tester (KH2 magic and Reflect native, m3 answer 6 (b)). `mpcost.rs`: a KH2 spell costs (KH2 cost / 100) of Sora's KH1 max MP, from the top down (the charge part at the next bar's worth, bar k worth k x 30), paid through KH1's payment `0x1402A5B60` (Steam) 30 units a call; Cure takes all (at least one unit). Base Sora's KH2 spells and Reflect; Drive Forms keep 1 MP. KH1's usable check `0x14028EFD0` reads our cost (spell data `+0x30`, capped 99) for the one command step (`CostPatch`). Checked (files): Fire 12, Blizzard 15, Thunder 18, Cure 255, Reflect 10 (03system `cmd`), KH2 Sora's MP 100 (`plrp`); Checked (decomp) `0x1402A5B60`, `0x1402A4C10`, `0x1402A4CB0`, `0x14028EFD0`. Open: KH1's menu still greys by KH1's cost. docs/kh1-magic-native.md §6.

### 2026-10-24: Stage 483 (testing branch: Reflect's effects at KH1 Sora's size)
- The tester (m16, 2 (a)): scale Reflect to KH1 Sora. `kh2dpx::kh1_sized`: Reflect's packages' elements' scale and position-only offset times `pose::kh1_size()` (the retarget's hip ratio, about 0.89, new `KH1_SIZE`), at each element's start. KH1's instance scale carries the header offset, Checked (decomp) `0x14031CEA0`. The barrier's attack shapes (ATKP 573) are not scaled (gameplay). docs §18.22.

### 2026-10-24: Stage 482 (testing branch: KH2's object frame on KH1's object root, through KH1's apply turn)
- The tester (m16): Reflect "(b) off to one side"; Firaga "it isn't changed" (one flame). KH1's 0x8000 frame read straight (`0x14029EA00`) is (back, up, -x); through the apply (t481's P) it is (-x, down, back), KH2's object frame, so bound object-root elements keep KH2's offset and turn: t454's quarter turn `C` is gone (it turned them 90° about Sora). Position-only offsets now go through P; under `C` their height was reversed, putting Fire 3's ring (elements 2, 6, 7, 10) 72 to 218 below his feet. Checked (exe/trace/files: `FIRE_3.mag` effect 2, VertexApLc `0x14033D330` / KH2 `0x1404D3E00` the same). Also changes Explosion's burst, the forms' root effects, 0x4001 / 0x4009 elements. `note_place` prints KH2's offset along the actor's rows. Assumption: 0x8005 reads the same frame. docs §18.21 (§18.17 superseded).

### 2026-10-24: Stage 481 (testing branch: Blizzard's shard turned as KH2's)
- The tester (KH2 effects in KH1's own effect system, m16): "blizzard is sideways". KH1's apply `0x1402ADB40` (Steam) takes its angles with `0x1402A8EF0` (x = atan2(-row1.z, row0.z), y = asin(-row2.z), z = atan2(row2.y, row2.x)), which isn't the inverse of KH1's X-Y-Z build `0x140323480`, so an element given M is drawn M·P (P: columns z, -y, x; P·P = 1). Checked (exe); the t478 log's object-root turn agrees, Checked (trace). Blizzard 3's shard is placed by its script's `trap_effect_set_dir` (KH2 trap 0/41, table `0x140753040`, `0x14041E100` -> `0x1403B7710`: +z along the direction; Checked (exe/decomp); traced on `BLIZZARD_3.mag`, Checked (files)). `kh2dpx::spell_place` now passes `at·P` (`KH1_APPLY_TURN`); Wisdom's shots and Reflect's contact flashes go the same way. Assumption (open): bone- and Keyblade-bound pieces are also drawn B·P. docs/kh2-effects.md §18.20.

### 2026-10-24: Stage 480 (testing branch: Frame Advance: KH2's camera on the mod's clock)
- The tester (Frame advance in the debug menu), on t465's question about KH2's camera timing its gaps in real time: "unsure about your questions ... do the check ... go ahead". `camera.rs`'s `now_ms` (its 300 ms "was ours", 5 s "untouched" and 150 ms follow-call windows, the trace and jerk logs) is now `frameadv::clock_ms` (real time less the time Frame Advance held the game), so a hold over 5 s no longer starts KH2's camera over behind Sora on the next step; subtractions saturate. Select while held stays blocked (no change). Checked (code): the windows in `camera.rs`. Assumption: no window there wants real time across a hold. docs/kh1-frame-advance.md.

### 2026-10-24: Stage 479 (testing branch: Config menu: the glint follows the cursor)
- The tester (Config menu rework): "the glowing cursor that follows each command is not following properly, it stays at the top. use the c decomp". The glint is KH1's `0x1402D9B40` (a 30 x 30 glow at a point), its target from `0x1402D9920 (win, frames)`: the selected label's top-right corner, y = window y + 71 + row x window `+0xF0`. KH1's Config list def has a row pitch of 0 (`0x1405463A0 +0x22`), so in KH1 itself it stays on row 0. Our lists' defs now carry the rows' pitch (26 + 2), so KH1's own call follows the cursor; on page 1, after KH1's update, `0x1402D9860 (8, x, y)` (Steam) puts it at the selected row's corner taken through the 92% scale. Checked (decomp) for both functions and the window fields, Checked (exe) for the def's pitch of 0. Assumption: the glint's size stays unscaled on page 1. Sub-lists stay full size (answer 1a). docs/kh1-config-menu.md §5.

### 2026-10-24: Stage 478 (testing branch: KH2's finisher casts on KH1's combo)
- **KH2's finisher casts on KH1's combo.** The tester (same thread). 57/60/63 as alternates on 0x36..0x38 / 0x3D, `kh2slots::pick_cast`; KH2's rule on KH1's count (`+0x41`, `+0x50` or Combo Master, length `+0xD4`), finisher sub and voice. Notes `docs/kh1-magic-native.md` §4b.

### 2026-10-24: Stage 477 (testing branch: KH2's casts in KH1's own cast state)
- **KH2's casts in KH1's own cast state.** The tester (thread kh2-magic-and-reflect-native). `kh2slots::CASTS`: KH2 56/59/62/65 and air 58/61/64/67 on KH1's requests 0x36..0x39 / 0x83..0x86, plus 0x3D / 0x8A; no template leave point. `combat::command` doesn't borrow; hook on KH1's spell start `0x14028F140` (Steam, Checked (exe)) runs KH2's script instead (`formmagic::kh1_spell_start`): KH1's MP paid, MAGC voice, Fire's second part dropped from the queue; `takeover::native_spells` releases at the clip's magic-cast event on KH1's time. KH1's voice group off at Battle Quotes KH2. Notes `docs/kh1-magic-native.md`.

### 2026-10-24: Stage 476 (testing branch: Finishing Plus's next finisher on a KH1 row)
- **Finishing Plus's next finisher on a KH1 row.** The tester (m14 4 (b)). Rows / requests 0x206 / 0x207 with every finisher slot (`pick_own`); KH2's window (range 50) as KH1's combo window (instance `+0x14` bit 22, `0x1402A2EC0`); KH2's finisher count kept. Notes §7.

### 2026-10-24: Stage 475 (testing branch: Aerial Recovery and Retaliating Slash on KH1 requests)
- **Aerial Recovery and Retaliating Slash on KH1 requests.** The tester (m14 3 (b)). Requests 0x205 / 0x204; `combomods::recover` (KH2's hop 100 / 10), `Character::would_take_row`; KH2's free state as KH1's "may be left" on our requests. Assumption: KH1's knock-back ends with flag 4. Notes §7.

### 2026-10-24: Stage 474 (testing branch: KH2's Square moves and Guard Break on KH1 rows of their own)
- **KH2's Square moves and Guard Break on KH1 rows of their own.** The tester (m14 2 (b)). `kh1rows`: Sora's table (`0x142D5CAB0`, block `+0xA9F0`, 66 rows, Checked (exe)) + rows 66.., requests 0x200.., own data (launch kind; Guard Break priority 0x62, `0x1402B4BC0`, Assumption); `scale_record` KH2's numbers; Square through KH1's Attack command (`0x140293480`); air fall queued; `kh2slots::air_root` (Finishing Leap). Counterguard left. Notes §7.

### 2026-10-24: Stage 473 (testing branch: KH2's hits on his body on KH1's rows)
- **KH2's hits on his body on KH1's rows.** The tester (m14 1 (a)). Vicinity Break 170 / Explosion 166 / Magnet Burst 197 slots (`kh2slots::body_strike`); `takeover::kh2_hits` (`RigAnim::pose_at`, `Strikes::strike_as`, info `+0x50`); KH1's sphere on Keyblade windows only; range 27 in `hurt`. Notes §7.

### 2026-10-24: Stage 472 (testing branch: Debug menu: the font in its own buffer)
- **Debug menu: the font in its own buffer.** The tester ("if the debug allocates for kh2 model then make sure its gone"). `fx12::FontBuf` 147 KB; the 24 MB texture arena only made by a model draw. Notes §18.11.

### 2026-10-24: Stage 471 (testing branch: Debug menu: KH1 Spells)
- **Debug menu: KH1 Spells.** The tester (a toggle for the KH1 magics KH2 changed). No KH2 effect starts for Gravity / Stop / Aero (t463 log; KH2 motions 62 / 65 have no trigger, Checked (files)); at Magic KH2 KH1's effects play on KH2's cast cut at frame 68. Per spell "KH2 cast" (default) / "KH1 cast", base Sora, ini `kh1_cast_*`; its page is on the main page. `formmagic::kh1_cast`. Notes §18.19.

### 2026-10-23: Stage 470 (testing branch: KH2 effects: 0x4009 on KH1's body centre plus KH2's 20)
- **KH2 effects: 0x4009 on KH1's body centre plus KH2's 20.** The tester ("try (c)"). 0x8005 plus KH2's kind-9 point minus its kind-1 (Sora 105 − 85, Checked (files)); KH1's body-centre height logged once. Notes §18.16.

### 2026-10-23: Stage 469 (testing branch: KH2 effects: Reflect's frame re-examined; spell pieces' start places logged)
- **KH2 effects: Reflect's frame re-examined; spell pieces' start places logged.** The tester ("reflect still is misplaced"). t454's frame holds (KH2 decomp `+0x40`, the old player's diag(1,−1,−1), motion 71's barrier at the hands, Checked (decomp / exe / files)). `note_place` logs each spell piece's start place beside KH2's numbers. Notes §18.17.

### 2026-10-23: Stage 468 (testing branch: KH2 effects: a spell's pieces in one DPD per file DPD)
- **KH2 effects: a spell's pieces in one DPD per file DPD.** The tester (Firaga missing flames). KH1's 10 texture slots per frame (`0x1401EDB00`, Checked (exe)); t463 log: level-3 spells needed 21..37. One DPD per file DPD as KH2's; split per effect when over. Assumption: the cause of the missing flames. Notes §18.10.

### 2026-10-23: Stage 467 (testing branch: KH2 effects: MatrixLoop (216) as KH1's)
- **KH2 effects: MatrixLoop (216) as KH1's.** The tester's t463 log (Blizzard's shard). Calc-only in both (KH1 `0x14032EBF0`, KH2 `0x1404C4540`, bytes equal but call displacements, Checked (exe)); `KH1_SAME`. Notes §18.9.

### 2026-10-23: Stage 466 (testing branch: KH2 effects: KH2's per-palette HD images)
- **KH2 effects: KH2's per-palette HD images.** The tester's t463 log (Thunder 3: 0xd231..3, 0xe651..2). KH2 `0x1404AECD0` (Checked (decomp)): each record of a TBP at first + 1.. ; `kh2dpx::palette_textures` after KH1's `0x14031EC10`; the builder keeps KH2's mask bits for own records. Notes §18.12.

### 2026-10-23: Stage 465 (testing branch: Frame Advance: the Camera row (Free: the camera moves while held))
- **Frame Advance: the Camera row (Free: the camera moves while held).** The tester ("toggle the cam movement on or off"). KH1's pad step runs outside the task list; 100000 `0x1401E5D90` steps the camera mode (`0x140506CE0`) from the live stick, round the held Sora, writing only the camera's state; 110000 builds the view from it (Checked (decomp), table Checked (exe)). Free: 100000 runs held through `camera_live` with the frame's scale (0 otherwise since t463), no Select (first person clears the lock-on and fades Sora), no R3 in first person; KH1's first-person fade `0x1425380F8` holds it. KH2's camera goes along (same scale). Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 464 (testing branch: Frame Advance: its rows on a debug menu page of their own)
- **Frame Advance: its rows on a debug menu page of their own.** The tester ("make frame advance a submenu and throw it plus frame advance option in there"). Main page: Frame Advance `>`; its page: Frame Advance, Held Sound, Game Frames (also on the Trace Recorder's page), Held Tasks. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 463 (testing branch: Frame Advance: every update held, every draw run)
- **Frame Advance: every update held, every draw run.** The tester ("focus on getting the live version working"). Runs while held: 10499, 110000, 129999, 135000 (Checked (decomp)); held: 163001, 190000 `0x1EB750`; frame scale `0x14233FBDC` and half `0x14233FBE0` 0 on held frames; 110000's zoom, 164004's pulses, the HUD data `0x1427F13D0`..`0x142866D18` and model colours put back each held frame. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 462 (testing branch: Frame Advance: the main list's tasks read one by one)
- **Frame Advance: the main list's tasks read one by one.** The tester ("look through the c decomp… do that for the rest of the values"). Every registration on the main list read (Steam decomp); 110000 is the view set-up (texture scroll, fog packet), not a camera; KH1's render queue `0x1401CB730` found. Held Tasks labels corrected. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 461 (testing branch: Frame Advance: Live only)
- **Frame Advance: Live only.** The tester ("the actual pause seems to hold the image in the frame buffer"). KH1's pause picture (task 190020, capture `0x14018B370`) and the Held Picture row removed; effects held by KH1's system stop as in t447. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 460 (testing branch: KH2's ability moves on KH1's rows at the combo style KH1)
- **KH2's ability moves on KH1's rows at the combo style KH1.** `native_ability` style-free, `Kh1Combo::here`, `kh2slots::pick_ability`. Notes §7.

### 2026-10-23: Stage 459 (testing branch: KH2's Combo Plus / Negative Combo picks on KH1's plain rows)
- **KH2's Combo Plus / Negative Combo picks on KH1's plain rows.** `Kh1Combo::serves`; no borrow at Base / Aerial Combo KH2. KH1's finisher is known by its request. Notes §7.

### 2026-10-23: Stage 458 (testing branch: KH2's turn toward the target in a swing on KH1's rows)
- **KH2's turn toward the target in a swing on KH1's rows.** Range 28 (`0x1403D4B10` / `0x1403D3A90` / `0x1403B9F70`, Checked (decomp)): up to 2 × the parameter an update toward `+0xDC8`, past 50 away. `kh2slots::turn_to_lock` from `takeover::physics`; target `takeover::kh2_primary_target`. Notes §7.

### 2026-10-23: Stage 457 (testing branch: KH2's edge stop on KH1's ground swings)
- **KH2's edge stop on KH1's ground swings.** Range id 29 (`0x14040D810` → `+0x18C` bit 23, Checked (decomp)): `0x1403B9090` sweeps the step + 0.2 (`0x14074B060`, Checked (exe)) × 90 and keeps only its vertical part. `kh2slots::edge_stop`, `shared::physics` ground step, `combomods::floor_gone`. 154 / 155 / 165 / 169 / 161 (Checked (files)). Plain swings still cut into the fall, as in KH2 (`0x1403D6430`) and KH1 (`0x14029A3C0`). Notes §7.

### 2026-10-23: Stage 456 (testing branch: fx12's vertex ring sized for the debug overlay)
- **fx12's vertex ring sized for the debug overlay.** The tester (3 (a)). 4 x 1 MB (was 4 x 6 MB, sized for the old effect player): one overlay draw is at most 18 lines and 2 boxes. The KH2 player model (off) would need it sized again. Notes §18.11.

### 2026-10-23: Stage 455 (testing branch: KH2 effects: KH2's flag 0x800 (alpha follows the owner))
- **KH2 effects: KH2's flag 0x800 (alpha follows the owner).** The tester (2 (a)). KH2 `0x1402C58F0` / `0x1402C6A10` (Checked (decomp)): while bound, the effect's alpha is the weapon's (0x4003..8) or owner's, 0 hidden. KH1: Sora's draw bit and `+0x60`, the weapon-out flag for the Keyblade; fade-out on its own level. Assumption: KH1's Keyblade alpha byte unchanged in play. Notes §18.18.

### 2026-10-23: Stage 454 (testing branch: KH2 effects: KH2's object frame on KH1's object root)
- **KH2 effects: KH2's object frame on KH1's object root.** The tester ("reflect is misaligned with sora"). KH1's 0x8000 frame (`0x14029EA00`: rows -+0xF0, -+0xE0, -+0xD0 of the skeleton pass's matrix, Checked (exe)) is KH2's object frame (x, -y, -z: the old player's, Assumption) a quarter turn about y; offsets and turns on 0x8000 / 0x8005 go through `KH2_IN_KH1_OBJECT` (copy, position read, script place). Notes §18.17.

### 2026-10-23: Stage 453 (testing branch: the coat as before t387)
- **The coat as before t387.** The tester (after t412): "before I prompted you to adjust the bake animations the coat was working how it should", "model ... also kh2 sora's skeleton". t387 put the jacket hem (73-74, 249-278) in `KH1_OWN`: it took KH1's own motion's relation (baked: the template; live: KH1's pass, hence t405 / t411). Out again: `KH1_OWN` 34 -> 16 (the shorts); the coat is s284..t386's (roots on KH2's pelvis, front panels a thigh share, joints at bind, `keep_clear`), live and baked. Checked (files): all KH2 motions (11906 frames) with KH1's run fed, the coat's 36 bones identical to t386's (t412: up to 27.7). KH2's coat ring (129-135 / 221-227, 70, from chest 68) carries no mesh. Test `local_own`. Notes `docs/kh1-secondary-motion.md` "The coat as before".

### 2026-10-23: Stage 452 (testing branch: KH2's first-person pitch rate and limits from KH2's exe)
- **KH2's first-person pitch rate and limits from KH2's exe.** The tester ("check the decomp"; the values are exe data). Read from the tester's Steam KH2 exe (SHA-256 `9002b2de...49ed`; its `0x140165e30` reads them, Checked (exe)): `0x140718bf4` 3° a frame, `0x140718bf8` / `0x140718bfc` ±70°. `kh2_camera::FP_PITCH_*` (was KH1's 80° up / 60° down). `0x140718bf0` (the turn) is 3° too, as ours. Notes `camera-model.md`, `kh2-pad.md`.

### 2026-10-23: Stage 451 (testing branch: KH2's camera springs once a frame at 30 fps)
- **KH2's camera springs once a frame at 30 fps.** The tester ("(a)"). `kh2_camera::settle`: one spring a frame, the rate halved when the step is under 2, whole from 2 (Steam KH2 `0x140165700`, Checked (decomp)); was two half springs. Keyboard / mouse at 30 fps closes about 91% of the gap a frame (was 75%); pad 17.5% (was about 16.7%). Notes `camera-model.md`.

### 2026-10-23: Stage 450 (testing branch: The mouse turns Sora in KH2's first person)
- **The mouse turns Sora in KH2's first person.** The tester ("check the decomp"). KH2 `0x1403a9250`: ±3° a frame on the direction bits, set from the mouse in a mouse frame (`0x14039c180`). `camera::first_person_mouse_x` (KH1 `0x140138b30`, right stick `0x14298cac0`); forms `first_person_turn`; base Sora: KH1's turn `0x14028c7e0` fed a full left stick `0x14298cad0` for its control step `0x1402a6220`, put back after. Steam only. Assumption: bits 20 / 21 are the right stick's x. Notes `docs/kh2-pad.md`.

### 2026-10-23: Stage 449 (testing branch: Config menu: KH1 in KH2)
- **Config menu: KH1 in KH2.** The tester (answers 1 Magic inside Combat, 2a, 4b, 5a, 6b). Page 1 has 9 rows: the button above Default; page 2 gone. Lists are KH1 windows (`0x1402E5A40` / `0x1402E3D10` / `0x1402E5E40`, KH1's help line). Back-out goes through the title tab `0x1402FADB0` (prologue `40 57 48 83 EC 40`, Checked (exe)). Steam only. Notes §6.

### 2026-10-23: Stage 448 (testing branch: Config list at 92% with KH1's window scale)
- **Config list at 92% with KH1's window scale.** The tester (answer 3a). Window `+0x6C` bit 0, `+0xA0`/`+0xA4` 0.92, anchor `+0xD8` 1, set in `0x1402FA210` (Checked (decomp)). Notes `docs/kh1-config-menu.md` §5.

### 2026-10-23: Stage 447 (testing branch: Frame Advance's Held Picture row: Live)
- **Frame Advance's Held Picture row: Live.** The t426 split with effects stopped by KH1's system stop (`0x142867368` bit 0, `0x142432130`, Checked (decomp) and (exe)), KeShpTail3 / 3X draw steps taken back, and 120000's sky / fades restored. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 446 (testing branch: Frame Advance holds the sound as KH1's pause does)
- **Frame Advance holds the sound as KH1's pause does.** `0x1400D2F80` (1 / 0) at hold start / end (pause states 3 and 4, Checked (exe)). Held Sound row.

### 2026-10-23: Stage 445 (testing branch: Frame Advance holds what KH1's pause holds and shows KH1's pause picture)
- **Frame Advance holds what KH1's pause holds and shows KH1's pause picture.** KH1's pause: mask 1, task 190020 (`0x14017FD60`), snapshot `0x14018B370`, redraw `0x140183CD0` (Checked (exe)). Every task without the pause flag held; our task at 190020 snapshots once per hold and redraws with spread 0. `0x1401ED6D0` also sets `+0xDB`, which hides effects (Checked (decomp)). Assumption: the snapshot sees the last frame. Held Tasks rows labelled. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 444 (testing branch: KH2 effects: the old player deprecated)
- **KH2 effects: the old player deprecated.** The tester ("the older player is to be deprecated"). `kh2fx` is the entry point to `kh2dpx`; player, `fx12::draw` / effect textures and `fx_test` removed; world-pass hook in `model` (installed only while the model is on); misses logged once. t430 log: the old player held 176 textures, 16,704 KB. Notes §18.11.

### 2026-10-23: Stage 443 (testing branch: KH2 effects: a form's package built once Sora could change into it)
- **KH2 effects: a form's package built once Sora could change into it.** `form::possible` (MP, data, Valor's Keyblade read without writing). Notes §18.12.

### 2026-10-23: Stage 442 (testing branch: KH2 effects: unbound position-only elements through KH1)
- **KH2 effects: unbound position-only elements through KH1.** Mode-1 start, place set from `by_position_now`, disarmed. Checked (exe). Notes §18.7.

### 2026-10-23: Stage 441 (testing branch: KH2 effects: logging HD images a texture record didn't find)
- **KH2 effects: logging HD images a texture record didn't find.** t430: Thunder 3's TBPs 0x3000/0x3020 untextured. KH2's extra per-record case (`0x1404AECD0`, 0xFFFD, Checked (decomp)) noted; not fixed. Notes §18.12.

### 2026-10-23: Stage 440 (testing branch: KH2 effects: only the elements that play are built)
- **KH2 effects: only the elements that play are built.** The Keyblade's package: its vanish flash (t430: 4 effects, 616,048 bytes). Notes §18.7.

### 2026-10-23: Stage 439 (testing branch: KH2 effects: KH2's 0x4009 point)
- **KH2 effects: KH2's 0x4009 point.** t430 log (Cure 3: 3, 4). KH2 = object turn at the first enabled kind-9 shape's centre (`0x1403B50F0`, Checked (decomp); kind 9 is read nowhere else); Sora's lift 105 on the root (Checked (files)). KH1 0x8000 with the offset moved (`effect_point_of`). Notes §18.16.

### 2026-10-23: Stage 438 (testing branch: KH2 effects: the spells' second-weapon points)
- **KH2 effects: the spells' second-weapon points.** t430 log (Fire 3: 9, 12; Blizzard 3: 7). KH2 0x4004/6/8 = owner weapon slot 1 for any package (`0x1403B5450` → `+0xD68`, Checked (decomp)); `spell_anchor` = `form_anchor`. Assumption: started only with two weapons. Notes §18.10.

### 2026-10-23: Stage 437 (testing branch: KH2 effects: position-only on KH2's object root)
- **KH2 effects: position-only on KH2's object root.** t430 log (Fire 3: 2, 6, 7, 10). KH1 0x8000 point at t 0 = actor `+0x10` (`0x14029EB20` last case, Checked (exe)); `posed` offset through the object frame. Notes §18.5.

### 2026-10-23: Stage 436 (testing branch: KH2 guard on his body in KH1's states; Flash Step)
- **KH2's guard on his body in KH1's states; Flash Step.** `kh2slots::body_guard` (ATKP `refact_self` 2, 00battle Checked (files)); `hurt` marks KH1's records no-effect with KH1's clang. Flash Step 169 native. Notes §7.

### 2026-10-23: Stage 435 (testing branch: KH2 Aerial Spiral and Aerial Finish on KH1's air combo)
- **KH2's Aerial Spiral and Aerial Finish on KH1's air combo.** Air ability rows through `Kh1Combo::plain` / `air_hop`, 192 on 0xCC / 0xCD, 194 on 0xCE. Notes §7.

### 2026-10-23: Stage 434 (testing branch: KH2 no-gravity ranges on KH1's air state; Aerial Dive)
- **KH2's no-gravity ranges on KH1's air state; Aerial Dive.** `kh2slots::no_gravity`; `shared::physics` air step holds the arc (KH1's step undone / KH2's arc not advanced). Assumption: the exe matches the OG rewrite. Aerial Dive 196 (46..54) native. Notes §7.

### 2026-10-23: Stage 433 (testing branch: KH2 Aerial Sweep from the ground natively)
- **KH2's Aerial Sweep from the ground natively.** Row 33 (191) through `kh2_hop`, as row 0, slots on 0xCC / 0xCD. Notes §7.

### 2026-10-23: Stage 432 (testing branch: KH2 Keyblade-only ability swings on KH1's ground combo)
- **KH2's Keyblade-only ability swings on KH1's ground combo.** `kh2slots::blade_swing` (weapon hits only, no body attack, guard, invulnerability or air/ground range; P_EX100.mset, Checked (files)); `combomods::native_ability`: Slapshot 162, Dodge Slash 163, Slide Dash 164 on 0xC8..0xCA, no borrow (root travel 50 / 300 / 700). 9 slots, about 1.5 MB. Notes §7.

### 2026-10-23: Stage 431 (testing branch: KH2 ledge hop checked in the t430 traces)
- **KH2's ledge hop checked in the t430 traces.** f48640 / f48888: row 1 about 40 from the edge, 21 high, 7.8/frame off it (t415 holds). f48311 / f49221: row 6 about 89 out (KH2's rule), 153's travel took him off. KH2's ground-mover edge stop (`0x1403B9090` / `0x1403BAB20`, decomp) is gated on `+0x18C` bit 23 (setter not found; Assumption: no swing sets it). No behaviour change. Notes §7.

### 2026-10-23: Stage 430 (testing branch: KH2 effects: KH2's body centre 0x4001 on KH1's 0x8005)
- The tester (option (a)). KH1 0x8005 (the first enabled kind-1 collision part's centre, object frame), Checked (exe);
  KH2's 0x4001 the same in KH2 (decomp); all packages; both heights logged once (KH2's: 85 for Sora). Notes §18.15.

### 2026-10-23: Stage 429 (testing branch: KH2 effects: KH2's pppSnoMdlDttNorm, the Drive sphere through KH1)
- The tester (option (a)). `kh2ppp::dtt_draw` (KH2 `0x1404CBBA0` / `0x1404CAD20`, Checked (decomp)) with KH1's pppKeMdlDtt calls;
  construct and reset KH1 145's. Assumptions: normal 1/4096, push x strength. Drive element 1 through KH1. Notes §18.14.

### 2026-10-23: Stage 428 (testing branch: KH2 effects: the forms' Keyblade points on KH1's blade binding)
- The tester (option (a)): 0x4003 / 5 / 7 KH1 blade binding on Sora; 0x4004 / 6 / 8 (KH2's second weapon) on
  `leftblade::left_owner` (refreshed after Sora's pass while bound; stopped when it goes, assumption). Notes §18.8, §18.13.

### 2026-10-23: Stage 427 (testing branch: KH2 effects: packages built when needed, lean, freed when not)
- The tester ("can we have the files generated at run time so we don't take up extra memory?"). `kh2dpx::manage`: wanted
  packages (Sora's for the `kh2_effects` moves + Drive and Revert, the Keyblade's, a form's from the Drive command to revert,
  spells and Reflect at Sora's levels) built in the background, entered at once, freed by KH1's removal `0x1400E2E00` once
  idle. One 32 MB region for all DPXs; one HD entry per DPD / TBP (`0x14031EC10`, Checked (exe)); common images one KH1
  texture each from `kh2fx_texcommon_dpx` (`0x14010C050`), shared. Sora's: 6.2 MB + 4.3 MB held -> 1.7 MB, none held. Until a
  package is built `kh2fx` plays its effects. Notes `docs/kh2-effects.md` §18.12.

### 2026-10-23: Stage 426 (testing branch: Frame Advance draws the level and the models while held)
- The tester's Held Tasks test (t399): 130000 (`0x2A23E0`) draws the models, 120000 (`0xD2830`) the level, and with 120000
  running characters can move; 100000 (`0x1E5D90`) seems to be the camera. `frameadv`: 120000 and 130000 run while held by
  default (`RUN_BY_DEFAULT`); 120000's own task lists (`0x14233E420` / `0x14233E7D0`: the animated map parts) are held
  instead, each a row on the Held Tasks page (`SUB_LISTS`). Assumption: those are what moved. Notes:
  `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 425 (testing branch: KH2 effects: the spell scripts' effects through KH1)
- Reflect, Wisdom and KH2 magic packages, Steam only: the old player's spell functions (start, place, end, kill, alpha,
  running) try KH1 first (`NATIVE_SPELL`); place through KH1's apply `0x1402ADB40` (Checked (exe)). What remains on `kh2fx`:
  notes §18.10-18.11.

### 2026-10-23: Stage 424 (testing branch: KH2 effects: KH2's pppAngleLoop and pppKeShpTail3X)
- `al_calc` (KH2 wraps angles above 0x10000) and `tail3x_draw` (KH1's draw, then the trail step taken back when held, paused
  or on a computed frame), Checked (decomp). Seven more programs added to `KH1_SAME` (SRandFV, SRandUpFV, KeMatSN,
  KeShpTail2X, PointLoop, ScaleLoop, DrawMdlLoop). Notes §18.9.

### 2026-10-23: Stage 423 (testing branch: KH2 effects: Valor's and Final's packages through KH1)
- Body and object-root anchors through KH1; weapon points stay on `kh2fx` (open question). Notes §18.8.

### 2026-10-23: Stage 422 (testing branch: KH2 effects: one DPX per package; the Keyblade's vanish flash and unbound elements)
- Unbound elements: KH1's start, then the binding disarmed (mode 0 + bind-once, Checked (exe)). Vanish flash at blade
  t = 0.5 (assumption, as the old player). `plug` flags only ever turn on. Notes §18.7.

### 2026-10-23: Stage 421 (testing branch: KH2 effects: the Drive motion's and Revert's flashes through KH1)
- Drive 2..13 and Revert 15..27 through KH1. Element 1 (KH2-only pppSnoMdlDttNorm, 238) stays on `kh2fx`. Notes §18.6.

### 2026-10-23: Stage 420 (testing branch: KH2 effects: position-only elements on KH1's binding mode 1; the rest of Sora's moves through KH1)
- KH2's flag 0x04 elements on KH1's binding mode 1 (start `0x1402ADC20`, Checked (exe)); after Sora's pass `posed` sets the
  record's offset to bone place + turn x offset - KH1's point. Guard Break, Finishing Leap, Aerial Recovery, 169 / 171 / 172.
  Notes §18.5.

### 2026-10-23: Stage 419 (testing branch: KH2 effects: KH2's pppVtMime calc; Vicinity Break and Aerial Finish through KH1)
- `kh2ppp::vt_calc`: KH2 skips a rate key below -1e38, and Sora's keys use it (KH1's would wreck the morph). KH1's draw,
  construct, reset and destruct kept. Assumption: KH2's calc from the §7.4 reading (not in the Steam decomp). Notes §18.4.

### 2026-10-23: Stage 418 (testing branch: KH2 effects: Sora's whole package built for KH1; Magnet Burst through KH1)
- Groups per motion (moveset frame triggers), Revert's set, then one per effect. Bind-pose bone table (§18.3). Gate:
  `KH1_SAME` programs (decomp comparison §18.2) or KH2's own plugged in; anything else goes to `kh2fx`, logged once.

### 2026-10-23: Stage 417 (testing branch: KH2 effects: the KH1 DPX builder for any effects)
- The tester ("KH2 effects in KH1's own effect system": "port every effect"). `kh1dpx::build_groups`: one DPD per group,
  from any DPDs of a PAX; each entry's program types and each texture's source DPD reported; a group over KH1's 10 / 6
  texture slots is left out and logged. Explosion's DPX unchanged (byte-identical offline). Notes `docs/kh2-effects.md` §18.1.

### 2026-10-23: Stage 416 (testing branch: KH2's line ahead, its floor rule and direction)
- The t409 traces: row 1 at a 120 drop, 50..70 from the edge, is KH2's. The test was KH1's (`0x1402C6E70`: any step more than
  half KH1's height down) along KH1's press direction. Now KH2's (`0x140172640` / `0x140172840`, decomp): only the end of the
  85 line (`attack_j_front` -5 + `+0x6F0` 90; `+0x150` = plyr, Checked), none or more than lift - height - 1 = 24 below his
  feet (lowered by a downhill line), laid on the floor's plane (KH1's `0x1401907F0`), along `Character::selector_aim`
  (`0x1404028D0`). KH1's set-up / mover (`0x14029A0B0` / `0x140195180`, bytes checked) move it. Assumption: flat on
  objects. Notes `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 415 (testing branch: KH2's hop out of the ground combo keeps its speed)
- Mr Bugs (t409 traces f3990..f5413: "next to a ledge will start a short jump attack"): every row-1 hop rose and fell in
  place, `+0x3C0` 0 from its first frame. KH1's control step (`0x1402A3300`, decomp) sets `+0x3C0` = `+0x20` x `+0x2C` (the
  stick, from `0x1402AFC20`) on a frame its state changed into 2. `shared::physics::ground_hop_carry` puts `kh2_hop`'s
  velocity back before KH1's physics step. The hop firing there is KH2's (a 120 drop 50..70 ahead). Notes
  `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 414 (testing branch: KH2's mouse camera)
- The tester ("(b)"): KH1 PC's mouse code is KH2 PC's (Steam KH1 `0x140138b30` = KH2 `0x1404e51f0`). Mouse frames: both
  sticks KH1's processed ones times `1 + 0.02 s`; the follow camera turns `max(|10x|,1)·3°` and zooms `8·max(|10y|,1)`
  (`0x140166100`); keyboard/mouse mode (`0x1422DB3E8`) lag 0.4 + (run-8)·0.0018 (`0x140165700`). Steam only; prologue
  checked. Trace `kh2cam_mouse_x/y`, `kh2cam_kbm`. KH1's own mouse camera (`0x1401e7320`) compared in `docs/kh2-pad.md`.

### 2026-10-23: Stage 413 (testing branch: R2 holds KH2's camera swing)
- The tester ("its R2"): bit 32 of KH2's held word is R2. `kh2_camera::Controls::hold`: the swing behind runs only when
  `auto != hold` (Steam KH2 `0x140178ae0` / `0x140165110`). `camera::r2_held`: Steam KH1's raw pad word `0x14298C9B4`
  bit `0x200` (keyboard too; assumption), else `RAW_BUTTONS`; Frame Advance's R2 stays hidden. Trace `kh2cam_hold`.
  Notes: `docs/kh2-pad.md`, `camera-model.md`.

### 2026-10-23: Stage 412 (testing branch: KH2's hood is on its torso)
- The tester: "the hoodie doesn't seem to be animated anymore". From the files: KH2's model has no hood standing off the
  back; the mesh behind the neck on 140 is the hair's, the hood is the top of the back on the torso 67 (68 above carries
  none, no motion turns it off 67). `KH2_SHOWN_ON` hood 72 / 73 -> 67 (`KH2_TORSO`): KH1's hood keeps its bind relation
  to the neck base (t407 had it turning with KH2's head). Assumption: that patch is the hood. Notes
  `docs/kh1-secondary-motion.md` "KH2's hood".

### 2026-10-23: Stage 411 (testing branch: KH1's own parts: a skipped bone holds our matrix moved)
- Mr Bugs (t409 picture, trace f20377): "his coat, his pants ... return to the map origin". Both of KH1's passes end by
  adding the world translation to every bone (after the world scale, if off 1), placed or not (`0x140392560`,
  `0x140391C70`, decomp); t405's `Sora::wrote` test took the blend's skipped parents as placed, so the hem and shorts
  were measured one world position off on every blend frame (each stop from walk to idle and the frames after).
  `pose::kept`. Assumption: the scale's side. Notes `docs/kh1-anim-slots.md` §1.6.

### 2026-10-23: Stage 410 (testing branch: KH2's combo rows checked in the tester's traces)
- His t409 traces f37236..f44266 (physics KH1, Base / Aerial Combo KH2): every air pick and per-row hop is KH2's rule
  (heights 21 / 17 / 21, 67..71 toward an enemy above; speeds at the row bounds or the gap's). Notes and two comments
  corrected: KH2's row 10 and KH1's air row 17 are for a target above, 184 / 152 / 154 only off the primary's line. No
  behaviour change. Notes `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 409 (testing branch: KH2's walk / run split)
- The tester ("yes"). Steam KH2 `0x1403a8fa0` / `0x1403aa700`: walk below tilt 0.8, run from it, no dead zone past the
  pad's. `run_threshold` 0.8 (was 0.5), `DEAD_ZONE` 0 (was 0.1); the ledge hang keeps 0.1 (`HANG_PUSH`). Notes:
  `docs/kh2-pad.md`.

### 2026-10-23: Stage 408 (testing branch: KH2's first person in KH1's first-person mode)
- The tester ("KH2 Camera fixes", option (a)): Select's first person is KH2's. KH1's mode 2 entries swapped
  (`first_person_mode` / `first_person_exit`, Steam `0x1401E2390` / `0x1401E2610`; Epic from the table's neighbours) only
  if both are KH1's own. Steam KH2 `0x140165e30`: eye at the position + `prty` subjective_offset, 350 ahead along the
  facing, right stick up/down pitch, R3 levels, Select out. Sora fades over 30 frames each way through KH1's alpha
  `+0x60` / `+0x64`. He stands (`move_tilt` 0) and turns 3 deg a frame. Assumptions: the pitch rate and limits (KH1's) and
  which stick turns him. Notes: `camera-model.md`, `kh1-kh2-camera.md` 9c.

### 2026-10-23: Stage 407 (testing branch: only what KH2's model shows)
- The tester: "(b) only do what kh2 does so its 1:1". `secondary::KH2_SHOWN_ON`: KH2's keyed bones with no mesh read as
  held to their mesh bone (hood 72 / 73 and spikes 143 / 159 / 161 on 140, flaps 113 / 115 on 110, 205 / 207 on 202). Hood
  with KH2's head, sleeves without flap motion or squash; t386's scale pass now carries none of KH2's (files).

### 2026-10-23: Stage 406 (testing branch: the belt chain, belt-front pieces and pendant set aside)
- The tester: "remove them from animation for now", keeping the hem and shorts ("2. (a)"). `KH1_OWN` 41 -> 34 bones; 5, 8,
  11-14, 87 hang as before t387, live and baked.

### 2026-10-23: Stage 405 (testing branch: KH1's own parts on KH1's blend pass)
- Mr Bugs ("Missing sleeves animation"): "standing and returning to neutral cause parts of soras model to stretch like
  crazy" (his trace f8434). KH1's blend pass (`0x140392560`, `pose_ik`) writes a non-IK bone only with bone-table bit 30
  (decomp); every `KH1_OWN` part's parent lacks it (files), so `own_from_kh1` measured the shorts, hem and belt pieces
  against our last drawing. `Sora::wrote` tells a bone KH1 didn't place; it is rebuilt from the one above by KH1's last
  relation (constants only, files), else the part keeps its last. Notes `docs/kh1-anim-slots.md` §1.6.

### 2026-10-23: Stage 404 (testing branch: KH2 effects: the old effect player retired for Explosion)
- The tester (option (b)): on Steam with the DPX built and KH2's programs plugged in, Explosion is KH1's alone (no `kh2fx`
  fallback); `kh2fx` keeps it only before the DPX is built, if plugging fails, or off Steam. Ini `kh2_fx_native` removed.
  What still uses `kh2fx`: notes §17.8.

### 2026-10-23: Stage 403 (testing branch: KH2 effects: bound elements take their bone on KH2's frame)
- The tester (option (a)). From the decomps: both bindings apply the matrix read on their last run (KH2 `0x1402C58F0`, KH1
  `0x1402AE910`), but KH2's first run reads first, and it reads while waiting; KH1's start reads before Sora's skeleton
  pass. `kh2dpx::posed` (from `pose`, Sora only) re-reads our continuous bindings' records with KH1's `0x14029EA00` /
  `0x14029EB20`: after the pass on the start frame, before it while waiting. KH1's own bindings untouched. Notes §17.7.

### 2026-10-23: Stage 402 (testing branch: KH2 effects: KH2's pppColor for KH2's effects only)
- The tester (option (a), KH1's effects unchanged): KH2's pppColor (Steam KH2 `0x1404C0F60`, bytes written every call,
  clamped at 32767) as our third entry (`kh2ppp::col_calc`); `kh1dpx` lists our DPX's type-43 words and `kh2ppp::plug`
  points only those at it (KH1's entry 43 checked, `0x14032B940`). Assumption: KH2's scale 1/128 (KH1's `0x1403B30B8`);
  KH1's construct `0x14032BAE0`. Notes `docs/kh2-effects.md` §17.6.

### 2026-10-23: Stage 401 (testing branch: KH2's per-row hop on KH1's air swings)
- The tester: "(a)", 1:1. KH2 starts each row's hop when it is used, in the air too (`0x1403D78D0`, the count kept).
  Aerial Combo KH2, KH2's plain air row playing on KH1's 0xCC / 0xCD / 0xCE: `combomods::air_hop` / `Run::AirHop`;
  `combat::exec` runs KH1's air row, then `shared::physics::kh2_hop` puts KH2's height and speed on KH1's air fields in
  place of the row's own lunge (8 / 12 / 4) and hop. No slot: KH1's own. A press in KH1's take-off frames (state 1)
  keeps KH1's row's hop. Notes §7.

### 2026-10-23: Stage 400 (testing branch: KH2's attack hop rule from the KH2 exe)
- t392's hop numbers were the OG rewrite's; Steam KH2 `0x1403D78D0` (Checked, decomp): the primary target, from his own
  kind-3 target shape's centre (Sora's: 70 up, radius 55, `P_EX100.mdlx`) to the target's (`0x1403D7520`); height
  bounded by PTYA `+0x14` / `+0x10` (+ jump height with flag 2); speed (gap less both radii + `distance_adjust`, not
  held at 0) over `sqrt(|2h / 0.40816328|)` (`0x1403B73F0`), bounded by `+0x18` / `+0x1C`, the exe's test order.
  `AttackParams::target_lift` / `target_radius` (`kh2_data::sora::target_shape`). Assumption: which player class is
  Sora's (several share the method pair). Notes `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 399 (testing branch: Frame Advance's Held Tasks page)
- The tester's t378 test: the picture is still black while held; he asked how to help log it. His t393 log lists the 22
  tasks held (all under priority 140000). `frameadv::tasks` / `toggle_task`: the debug menu's new Held Tasks page (main
  page) lists them (priority, function RVA, Held / Runs); one switched to Runs runs while the game is held (this session),
  each switch logged with the list running. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 398 (testing branch: KH2 effects: Explosion's burst (element 48) through KH1)
- KH2 0x4000 to KH1's object root 0x8000; flag 0x02 (bound until started) to KH1's bind-once bit 0x10000. Needs both
  programs plugged in. Assumption: KH1's 0x8000 frame is KH2's object frame. All of Explosion now plays through KH1's own
  effect system on Steam. Notes §17.5.

### 2026-10-23: Stage 397 (testing branch: KH2 effects: KH2's pppEiDrawShipoly through KH1's renderer)
- Step 4. The ribbon ported (Steam KH2 `0x1404B9C00` / `0x1404B9260` / `0x1404B8380`: construct, reset, destruct, calc,
  the spline, the draw), drawn with KH1's GS setters, shader 0x2B, camera `0x14031EAA0`, vertices `0x1400F3040` and draw
  `0x1400FB9C0`; the spline matches `kh2-fx`'s to 4e-6. Not ported: the scroll-draw variant, spaces 2 and 3. Notes §17.3.

### 2026-10-23: Stage 396 (testing branch: KH2 effects: Explosion's elements 44 and 45 through KH1)
- Anchors from KH2 bone codes: 0x4003 / 5 / 7 to KH1's blade point (code 0x30, t 1 / 0 / 0.5, binding mode 3 `0x1402ADD60`,
  t the 5th argument, a float on the stack, Checked (exe)); KH2 67 to KH1 70 (same bind frame). They play only once
  KH2's MegaBirth is plugged in, else through `kh2fx`. Checked (files): KH2 170 / 78 and KH1 93 / 198 match within 0.05,
  so the wrist flashes need no turn conversion. Not done: flag 0x800 (alpha with the weapon). Notes §17.4.

### 2026-10-23: Stage 395 (testing branch: KH2 effects: KH2's pppRyjMegaBirth in KH1's program machine)
- Step 3 (the tester: option (a), port it). `kh1dpx` lists KH2-own program words (`Kh1Dpx::programs`); `kh2ppp::plug`
  points them at our 0x50-byte entry after KH1's relocation (`0x14031C2F0`), after checking KH1's entry 201. It is KH2's
  calc, step, birth, update and draw (Steam KH2 `0x1404BC3B0`..`0x1404BC710`) on KH1's instance, drawn through KH1's
  shape draw `0x14031E210`, with KH1's construct and destruct. Steam only. Assumptions: two operands the KH2 decomp
  drops (spin as turn-then-scale; `+0x1B0` the root's axis lengths). Notes §17.1-17.2.

### 2026-10-23: Stage 394 (testing branch: KH2 effects: bound elements start on KH2's frame)
- The tester Mr Bugs (`docs/tester-requests.md`, "KH2 effects in KH1's own effect system": "exact to kh2"): `kh2dpx::apply`
  takes KH1's 2-frame extra wait for bound elements (wait <= -2 x `0x14233FBCC`, `0x1401ECB50` / `0x1401ECF30`) off the
  wait it writes; both games test the wait before counting it down, so the start frame is KH2's. KH1's binding task
  (`0x1402AEDA0`, 139999) runs before the effect update (140000). Notes: `docs/kh2-effects.md` §16.2.

### 2026-10-23: Stage 393 (testing branch: KH2's drop-ahead row and the finisher tests' status bits)
- From the KH2 decomp (Steam): the "blocked line" is the mover's no-floor flag: row 1 on no floor ahead (`0x140402D20` /
  `0x1403BA750`, bit 0, `0x140172640`): `drop_ahead`, tested with KH1's sweep `0x1402C6E70` (Steam only) at attack_j_front +
  radius (assumption: KH2's length field). `+0x194` bit 5 (the finisher chain, assumption) and Berserk Charge during MP
  Charge (`+0x1DC` bit 11 = ability 107, `0x1403C1D10`) in `combo::condition`; a normal row under the combo length passes
  whatever the finisher count (the exe; the OG rewrite refused it). KH1 has no MP Charge, so Berserk Charge stays off.

### 2026-10-23: Stage 392 (testing branch: KH2's hop rows on KH1's air state)
- The tester: KH2's rows that hop to a target above. KH2's plain hop rows (0: 182, 1: 181): KH1's state 2 (`0x1402B1960`),
  count 1, its air swing row with KH2's motion, then KH2's height and speed on KH1's air fields
  (`Character::would_take_launch`, `Launch`, `shared::physics::kh2_hop`; the hop numbers from the OG rewrite's rule, not the
  KH2 exe). Needs the Aerial Combo style on KH2 too, else KH1's row is kept and logged.

### 2026-10-23: Stage 391 (testing branch: KH2's row selection on KH1's air combo)
- The tester: the same picker on the air combo. KH2's pick plays on KH1's air requests 0xCC / 0xCD / 0xCE (slots for 181, 182;
  183, 184, `kh2slots::swing_rows` with `air`); KH1's air rows 18 / 17 / 16 / 15 (`0x1402A6C70`) give way as on the
  ground (`Kh1Combo`), each row's request read from KH1's row table. KH1's air lunge and hop kept (asked).

### 2026-10-23: Stage 390 (testing branch: KH1's own ground rows give way to KH2's pick)
- The tester Mr Bugs (`docs/tester-requests.md`, "KH2's combo row selection", option (a)): at Base Combo KH2, where KH1's
  ground picker (`0x1402A6EE0`, Steam) took a row of its own (dash 0 / 1, row 2, 9 / 10, ability finishers 3..7, or the
  wrong kind) and KH2's pick is a plain ground swing or finisher, `combat::exec` runs KH1's plain row 13 / 11 or 8 instead
  (`combomods::kh2_row`); after a dash row the direction's y is put back to 0. His t374 trace f27098: KH1's dash (0xD0)
  where KH2 took row 5. Sora's full PTYA table (staged `00battle.bin`), KH1's row runner (`0x1402A68F0`) and row 2 (a
  drop ahead, `0x1402C6E70`; t374's notes called it a jump at a target above) in `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 389 (testing branch: KH2's walk and run baked)
- The tester: bake KH2's walk and run to compare. `kh2slots::CYCLES`: requests 0x1 / 0x2 are kind 3, three slots each on
  KH1's 2-4 / 5-7 (KH2's field cycle on the field slot, its battle cycle on the other two), redirected at the first;
  `Play::Slot` carries the field flag. Under Misc Animations. KH1's motion rate `+0x284` is set to 1 by `0x14029EF50`;
  otherwise only by event-script commands (decomp). Notes: `docs/kh1-anim-slots.md` §8.

### 2026-10-23: Stage 388 (testing branch: no hair switches)
- The tester: KH2 has no physics, its secondary parts are keyed, so the switches go. `hair_physics`, `kh2_hair` and
  `jiggle.rs` removed; KH2's animation of the hair, hood, sleeves, necklace and chain always runs on KH2's poses (the OG
  rewrite has no cloth or hair physics; no constraint in KH2's motions drives them). The keychain on a Dream weapon or the
  wooden sword keeps KH1's rest pose.

### 2026-10-23: Stage 387 (testing branch: KH1's own parts take KH1's own animation of the move)
- The tester: a baked motion for KH1-only parts, with KH1's animations as the reference. `retarget::KH1_OWN` (41 bones:
  belt chain, belt-front pieces, shorts, jacket hem, the necklace's pendant), `Retarget::set_own` / `own_relation`, the
  belt chain carried with its own links: they take KH1's own motion of the move KH2's stands in for, stretched to KH2's
  length. Live from KH1's pass (`pose::own_from_kh1`); baked from the template (`Baker::poses`, its slot from
  `kh2slots::bake_one`). Test `local_own`. Not done: matching the stride phase within a cycle.

### 2026-10-23: Stage 386 (testing branch: KH2's squash and stretch)
- The tester: carry KH2's scale keys. `Secondary::scales`, `secondary::apply_scale` (live, last step before the write),
  bake scale channels kinds 1-3. Scale is multiplied down the bones and stretches only each bone (KH1 `0x1403900B0` /
  `0x1401DB310` / `0x140394CF0`, decomp; KH2's side from the OG rewrite's evaluator). Today it carries only the sleeve
  flaps' scale (no other matched KH2 bone keys scale).

### 2026-10-23: Stage 385 (testing branch: KH1's necklace follows KH2's; the KH2 -> KH1 part list)
- The tester Mr Bugs (`docs/tester-requests.md`, "Missing sleeves animation"): the hem and shorts from KH2, only parts KH1
  has, and a list of every part shared between the models. From the files: KH2's model puts each vertex on one bone;
  the hood (72 / 73), sleeve flaps (113 / 115 / 205 / 207), spikes 143 / 159 / 161, the chest ring 129-135 / 221-227 / 70
  (KH1's hem layout), the trousers and the pelvis pieces carry no vertices, so the hem and shorts aren't copied from them.
- `Parts::necklace`: KH1 86 <- KH2 165 (which carries its mesh), measured against the chest, held off the chest. Full
  KH2 -> KH1 list in `docs/kh1-secondary-motion.md`.

### 2026-10-23: Stage 384 (testing branch: the form's run speed in KH2's camera rates)
- `camera.rs` passed a fixed run speed of 8; KH2 reads the player's `+0x12C`, so faster forms swing and follow
  faster. It passes the current form's KH2 run speed (or Sora's). Assumption: `+0x12C` is that run speed (the crate's
  notes). Notes: `docs/kh1-kh2-camera.md`.

### 2026-10-23: Stage 383 (testing branch: KH2's camera reset puts the eye on its goal; a cut while moving)
- `0x140165110`: a reset also puts the eye on its goal (only the turn eases); with the left stick held the cut
  (`0x140164520` with its cut flag, then `0x140164d70`) places the look point and eye with no spring. Which fields are
  the rendered points is inferred from that copy. `kh2-camera`. Notes: `docs/camera-model.md`.

### 2026-10-23: Stage 382 (testing branch: KH2's camera turns and zooms by the right stick's whole tilt)
- From the decomp (Steam KH2): the turn tests each axis's direction bit (`0x140178ab0` / `0x140178b10`, zoom
  `0x140178b50` / `0x140178b80`) and moves by the stick's whole tilt (`+0x2c`), so a full diagonal turns 3 deg and zooms
  8 a frame together; the follow camera zooms after a reset, the lock-on camera (`0x140176c70`) turns and zooms first.
  The swaps are option bits (`0x1402e5330`, `0x1402e5350`). Assumption: the direction masks (exe data) are the right
  stick's x / y bits. Notes: `docs/camera-model.md`.

### 2026-10-23: Stage 381 (testing branch: KH2 movement reads the stick KH2's way)
- `takeover::move_tilt` had its own round 20% dead zone (labelled KH2's, it wasn't); it and `stick_dir`'s fallback use
  `character_core::pad::stick` now, so gentle tilts walk Sora as in KH2. Open: KH2's walk / run split (`run_threshold`
  50% is a hypothesis) and the controller's own 0.1 `DEAD_ZONE` on top of the pad's.

### 2026-10-23: Stage 380 (testing branch: KH2's camera reads the sticks KH2's way)
- The tester: KH2's stick dead zone 1:1. `character_core::pad` (`stick`, `stick_vec`) follows Steam KH2 `0x14039c180`
  (60 of 127.5 off each axis, over 67.5, a direction and a tilt clamped to 1); `camera.rs` reads the pad's sticks
  that way and `STICK_DEAD` (15%, camera only) is gone. KH1's own processed sticks are built the same way
  (`0x14028b4f0`, dead zone 60 from `0x14028afc0`) and stay the stand-in for the keyboard and mouse. Notes:
  `docs/kh2-pad.md`.

### 2026-10-23: Stage 379 (testing branch: KH2's camera swing-behind test on the stick's direction)
- t373 correction (the tester asked to confirm in the KH2 decomp). From the decomp (Steam): `0x14039c180` keeps the left stick
  as a direction plus a tilt (`0x140142080` normalizes it), so `0x140165110` skips the swing within 25.8 deg of straight
  back at any tilt. `kh2-camera` tests and scales by that direction times the tilt (tests). Notes: `docs/camera-model.md`,
  `docs/kh1-kh2-camera.md` 9a.

### 2026-10-23: Stage 378 (testing branch: Frame Advance draws the held frames)
- The tester's t377 test: the game state held, but the picture was black on held frames (only the frames that ran
  showed). t372 held every task not flagged for a pause, and most of KH1's draws aren't flagged (from the decomp's
  registrations: flagged are 140000 and the model pass 160000, not the map, effects or HUD); KH1's pause shows the
  last frame instead.
- `frameadv::hold_tasks`: only tasks before KH1's first draw pass (priority under 140000: actors 80000, camera 110000,
  scripts and the other updates) are held; every task from 140000 on runs. The held tasks are logged once. Notes:
  `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 377 (testing branch: Frame Advance's period key hidden from KH1)
- The tester's rule (period key works the same way as R2): KH1 no longer sees the period key while Frame Advance is
  on. `frameadv::install` hooks KH1's keyboard mapping (Steam `0x1400D7E80 (input object)`, prologue
  `40 56 41 56 48 83 EC 48` read in the Steam exe), which clears DirectInput's period byte (0x34) in KH1's two key
  buffers (input `+0x12A0` / `+0x13A0`, filled by `0x1400DA9F0`) before KH1 maps keys to its pad. Steam only, like
  the rest of Frame Advance. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 376 (testing branch: KH2 effects through KH1's own effect system, step 2: the element layer)
- The tester Mr Bugs (`docs/tester-requests.md`, "KH2 effects in KH1's own effect system"): KH2's element features in
  KH1's own effect system, not a layer beside it. Notes: `docs/kh2-effects.md` §16.
- From the decomp (Steam): KH1's effect instance has a start wait (`+0xD0`, game frames; `0x1401ECB50` /
  `0x1401ECF30`), a colour its draws multiply by (`+0x30..+0x3C`), an ended byte (`+0x91`) and a scale (`+0x20`);
  the root is the instance's times the particle header's local transform (`0x14031CEA0`). `0x1401EC470 (owner)` stops
  the instance whose `+0xB0` is that element.
- `kh2dpx`: Explosion's elements 46 / 47 get the KH2 element's wait (+0.1 bound) and scale on KH1's instance; their
  offset and half turn are folded into their own copy of effect 33's particle data (`kh1dpx::build_with`, numbers
  `0x700 | element`); KH2's motion changing (0x20) or Sora's object going (0x200) ends them the KH2 way: 0x40 fades
  the instance alpha over the fade-out frames, then KH1's stop (`0x1401EC470`, Steam), 0x80 sets the ended byte
  (`kh2dpx::tick` / `owner_gone` from `kh2fx`). `kh2_fx::math::xyz_i_of` (inverse of `euler_xyz_i`).
- Found, not yet used: KH1's object-root code 0x8000 and its point along the Keyblade (bone 0x30, t, binding mode 3
  `0x1402ADD60`) for KH2's 0x4000 / 0x4003 (elements 48 / 44 wait for steps 3-4).
- Kept KH1's: a bound element starts at wait <= -2 frames (KH2 has no such rule; asked).

### 2026-10-23: Stage 375 (testing branch: KH2's sleeve animation on KH1 Sora)
- The tester Mr Bugs (`docs/tester-requests.md`, "Missing sleeves animation"): under KH2's moves Sora's sleeves didn't
  move. His t371 runs (KH2's run, then KH1's) log no bones; the cause is in the files: KH1's sleeve chains (128-143,
  232-247, four per arm from the upper arm 93 / 198) were in no retarget part and not in `secondary`, so they kept
  their bind relation to the arm. Both games key their sleeves by hand (KH1: 59-93 motions; KH2: root 110 / 202 and
  flaps 113, 115, 205, 207, no constraints in any motion).
- `secondary.rs`: `Parts::sleeves` (in `ALL`, `covers` 128-143 / 232-247). Each chain's shoulder joint takes KH2's
  sleeve root's (111 / 203) model-space turn, carried by the swing from KH2's upper arm onto KH1's; the joint down the
  sleeve takes the flap's turn relative to that root. Four KH1 chains blend KH2's two flaps by place round the arm
  (front / back 98%, under / top about half). Live (`pose`) and in the bake (`Baker`), with the hair's switches.
- Checked on the files, all 89 KH2 motions: no NaN, lengths kept, links within 0.03 deg of KH2's, sleeves within 29 deg
  of the arm (KH1's own motions: 41 deg). Not carried: KH2's flap scale keys. Notes and the other parts still rigid
  (shorts panels, hem, belt chain, necklace): `docs/kh1-secondary-motion.md`.

### 2026-10-23: Stage 374 (testing branch: KH2's row selection on KH1's ground combo)
- The tester Mr. Bugs (`docs/tester-requests.md`, "KH2's combo row selection"): KH2's row selection, so the right swing
  plays for the target's distance; the swing order was fixed (0xC8 / 0xCA 151, 0xC9 152, 0xCB 154).
- From the decomps (Steam): KH2's selector `0x140402D20` (row checks `0x140402750`, finisher / normal tests
  `0x1403D7780` / `0x1403D7850`, range `0x1404029F0` -> `0x1404027F0`, walk with anti-repeat `0x140402C50`, row 9 / 5
  fallback) is `character_core::combo::select`, except the range gap, which the exe never lets go below 0
  (`combo::in_range` now clamps it). KH1's Attack step `0x1402A7510` and ground picker `0x1402A6EE0` mark and walk its own
  rows the same way; its plain swings are rows 13 / 11 (0xC8 / 0xC9) and its finisher row 8 (0xCB).
- `Character::would_take_after`: the host's playing motion goes into the anti-repeat (`kh2_would_take` takes it;
  stage 312's "always row 5" log). `combomods::select_swing`: at Base Combo KH2, a pick that fits KH1's plain swing or
  finisher (ground, no hop, no ability) points that request at our slot of the picked motion (`kh2slots::pick`, before
  KH1's row requests it); `kh2slots::swing_rows` builds a slot for each such motion per request. KH1's row, hits and
  damage stay KH1's. `shared::swing_travel` takes the playing slot's motion. Notes: `docs/kh1-anim-slots.md` §7.
- Not changed: KH1's dash / jump-at-target / ability-finisher rows, KH2's hopping rows, the air combo.

### 2026-10-23: Stage 373 (testing branch: KH2's camera no longer circles Sora when he runs toward it)
- The tester Mr. Bugs (`docs/tester-requests.md`, "KH2 Camera fixes"): running backwards turned the camera and Sora in
  a circle. His trace f571370: left stick straight down, KH2's camera yaw and Sora's yaw both turned 0.0175 a frame
  (frames 571250-571262, 571287-571370): the swing-behind formula, 1 degree a frame.
- Cause: `camera.rs` passed KH1's stick (y positive pulled back) to `kh2-camera`, whose y is forward, so its "not pulled
  straight back" test read straight back as forward. `camera.rs` now negates y.
- `kh2-camera`: the swing test and rate use the stick's own components, not its unit direction. Checked in the exe
  (Steam KH2 follow update `0x140165110`, pad sticks `0x14039c180`): no swing while the pad's y is 0.9 or more pulled
  back; diagonally back or a lighter tilt still swings, as in KH2. Tests for both.
- Not changed: KH2's per-axis stick dead zone (60 of 127.5, `0x14039b7f0`) against ours (round 15%). Notes:
  `docs/camera-model.md`, `docs/kh1-kh2-camera.md` 9a.

### 2026-10-23: Stage 372 (testing branch: Frame Advance holds the actors)
- The tester's t371 test (`testers/frame-advance-in-the-debug-menu`, 8 traces at 30 / 60 fps, KH1 and KH2 moves):
  only effects held and stepped; Sora's physics frames went on every frame while the mod's clock stood still. The
  pause machine (`0x14017F150`, every frame before the task list) sets the task mask each frame itself, so t371's
  write was overwritten; `0x1401ED6D0` pauses the effect instances (pool `0x142678280`), not models.
- `frameadv`: while the game is held, every task of KH1's main list (`0x142867380`, 0x18-byte tasks: `+0` in use,
  `+2` flags, `+8` next, `+0x10` function) in use and not flagged 1 (the flag KH1's pause runs) has its function
  swapped for `frameadv::skip` (does nothing, keeps the task), and gets it back for a frame that runs; the list is
  walked every held frame, so new tasks are held too. The actors are one of those tasks (`0x140298940`, priority
  80000, runs the actor list `0x142D5AB00`). The mask is no longer written. Data writes only, no hook. Notes:
  `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 371 (testing branch: Frame Advance in the debug menu)
- The tester Mr. Bugs (`docs/tester-requests.md`, "Frame advance in the debug menu"): a debug menu row that holds the
  game; R2 or the period key plays one frame, held 1 s plays at full speed until let go; works at any fps; R2 does
  nothing else while it's on; a button held over several steps is one press, one let go is released on the next step;
  the trace's seconds count game frames, with a Game Frames row on the Trace Recorder page (option b: off by default,
  turned on with Frame Advance). Notes and addresses: `docs/kh1-frame-advance.md`.
- `frameadv.rs` (new): from KH1's frame end (`debugmenu::frame_end`, after the frame's task list) the next frame is
  held or run the way KH1's pause holds it: the task mask (Steam `0x142867370`: 1 = only the pause-flagged tasks,
  0 = all) and the model objects' pause (`0x1401ED6D0`). KH1's pacing runs every frame, so a stepped frame has the
  frame scale of the fps setting. Before a stepped frame KH1's pad record's held buttons and repeat timers are put
  back to the last frame that ran. Left to KH1 while its pause machine is up. Steam only (the row says so elsewhere).
- R2 is taken off every pad record while it's on (`probe::pad`, and `takeover::RAW_BUTTONS`); the period key
  (`GetAsyncKeyState`, game window in front). Not done: hiding the period key from KH1's keyboard mapping.
- The mod's clock (`takeover::now_ms` -> `frameadv::clock_ms`) stops while the game is held; the draw's "posed
  lately" checks (`model`, `pose`, `leftblade`) take `frameadv::age`; `takeover::paused` is false while held.
- `trace`: Game Frames (`trace_game_frames`, `trace::game_frames`): the clip's window is `seconds` x 60 ticks
  (`Row::ticks`), else real time (`Row::wall_ms`, new). Select + Triangle writes the trace while held
  (`trace::frozen_combo`). Debug menu: main page Frame Advance row, Trace Recorder page Game Frames row.

### 2026-10-22: Stage 370 (testing branch: KH2's HD textures matched by the right header word)
- The user's t369 run: Explosion still has the squares. The log: the DPX was entered in KH1's loaded files, but with 0
  HD textures, every texture "none". The asset headers' `+0x20` words (P_EX100: `0x850A0`, `0x87120`, ...; texcommon:
  `0x424D0`, `0x62550`, ...) each step by the asset before's size (8320, 131200 bytes): the asset's own place in the
  stream, not the replaced data's offset (stage 369's assumption, wrong).
- `HdAsset` keeps both words: `pkg_offset` (`+0x20`) and `offset` (`+0x24`), and textures are matched by `+0x24`.
  Checked in the exe: KH1's own loader reads the same header's `+0x24` word as an HD entry's key (`0x1400D53F0`, low
  24 bits kept, `0x1400E1680`). The log line now lists each asset as "name for <offset> (size, at <pkg_offset>)".

### 2026-10-22: Stage 369 (testing branch: KH2 effects through KH1's own effect system: their HD textures)
- The user's t368 test: Explosion's wrist flashes showed as squares at Sora's body, the rest fine. The log: the DPX was
  built and relocated and both elements started, so placement and programs ran and only the textures failed.
- From the decomp (Steam): KH1's PC port never draws a DPD's PS2 texture data. Its DPD texture pass (`0x14031EC10`,
  one texture per TBP) asks the HD texture lookup (`0x14010C050`) for each record's pixels (`+0x20`): the loaded file
  holding the pointer (`0x1400E2B20`: KH1's loaded-file list, each entry's data `+0x658`, size `+0x640`), then that
  file's HD entry (`+0x670` list) whose `+0x54` is the pixels' offset in the file (the offset keying is on: set at
  start in `axaAppMain`). Our buffer was in no file, so there was no texture. KH2's PC port does the same with its
  own HD images (DDS, DXT5, as KH1's), stored with each file.
- The user chose option A (native): the DPX is entered in KH1's loaded-file list the way KH1 enters a file it loads
  (`0x1400E1680` / `0x1400E2F60`: a 0x688-byte resource entry, its data, size, name, MD5 (KH1 keeps a file's textures
  while a file with that MD5 is loaded), and 0x78-byte HD entries), each texture record given the KH2 HD image KH2
  draws for it. KH1's own loader makes the textures. Vtables are copied from KH1's own entries, memory comes from the
  game's CRT (`malloc`, `_aligned_malloc`), the list from the lookup's own first instruction (no new address).
- `kh2_formats::egs::Archive::read_remastered_all` / `kh2_data::Install::read_remastered_any`: a file's HD assets with
  each header's `+0x20` word. **Assumption** (OpenKH's `RemasteredEntry.Offset`): the offset in the original file of
  the data the asset replaces; the log lists each asset and which texture it matched, or "none".
- `kh1dpx::build`: also rewrites each texture record's own TEX0 (`+0x10`, which KH1's records keep equal to `+0`/`+2`),
  and returns each texture's origin (own or common, its record's offset there, its TBP, its offset in the DPX), the
  source DPDs' places and the DPX's MD5; `texture_records(dpd)`. A texture takes the HD image of its source TBP (the
  asset at any record with that TBP: one image per TBP in both games).
- **Assumption**: KH1 adds to its loaded-file list on the game thread (ours is added there, on the first native start).

### 2026-10-22: Stage 368 (testing branch: KH2 effects through KH1's own effect system, step 1)
- The user: port KH2's effects into KH1's systems so any move can use them; ladder step 1 (native), bones by option A
  (as the base attacks), Explosion first. Plan and checks: project notes `kh2-effects-in-kh1.md`.
- Checked (Steam decomps and the files): KH1 and KH2 share the effect engine (DPX `0x82`, DPD `0x96`, programs 1..235
  the same by name and number; KH2 adds 236..242) and the 30 Hz effect clock. What differs is texture memory: KH1's
  own-texture window starts at TBP `0x3800` / CBP `0x3D40` (10 / 6 slots), KH2's at `0x3000` / `0x3E00` with its common
  textures below.
- `kh2_formats::kh1dpx::build(pax, texcommon, effects)`: a KH1 DPX of some effects of one KH2 DPD. Keeps their particle
  datas only; the textures they draw with (the DPD's own and KH2 common ones) packed from KH1's window (textures sharing
  a TBP get the largest one's space); shapes' and models' TEX0 moved; texture masks rebuilt; programs above 235 set to
  KH1's dummy program until plugged in. Explosion (effects 31..34): 4 particle datas, 12 textures, 6 TBP / 2 CBP slots.
- `kh2dpx` (new): builds Explosion's DPX in the background from the KH2 install; on its first use KH1 relocates it
  (`0x1401E0080`), and elements 46 / 47 (effect 33, KH2 bones 170 / 78) start with KH1's start-on-bone (`0x1402ADCF0`)
  on Sora's KH1 bones 93 / 198 (`0x2000 | bone`). KH1 steps, binds, textures, draws and ends them; nothing of ours runs
  per frame. Kept inside one 32 MB handle region. `kh2fx::event` hands those two elements over first; while the DPX
  isn't built, or KH1 has no free element, `kh2fx` plays them as before.
- Ini `kh2_fx_native` (on): off = `kh2fx` plays everything as before.
- Not yet (step 2): the elements' delay, fade-out, end-with-motion and their half turn about z (KH1's bone binding
  doesn't take an element's rotation). Explosion's elements 44, 45 and 48 still play through `kh2fx` (they need KH2's
  `pppRyjMegaBirth` and `pppEiDrawShipoly`, steps 3 and 4).

### 2026-10-22: Stage 367 (testing branch: the baked swing travel at KH2's full distances)
- The user: the baked travel at KH2's full distances, to match KH2's travel (t365 scaled it by the retarget's hip
  ratio, a choice made without asking). Keeping the swing-travel feed off for baked swings is fine (the user).
- `Baker::poses`: the travel is KH2's root travel along the ground as it is, no ratio. Sora's placement has scale 1
  (t364's KH1 trace: swing 98's H10 steps 1.3 and 6.32 a frame, moved 1.30 and 6.32), so H10 moves him 1:1. The body
  carries the same, so the model stays on him; his feet may slide a little on KH1's shorter legs.

### 2026-10-22: Stage 366 (testing branch: t365 builds)
- t365's build: E0592, `Retarget::hip_ratio` defined twice; `Retarget` already had it. The added copy is removed.

### 2026-10-22: Stage 365 (testing branch: KH2's swing travel baked into the motion, KH1 moves him by it)
- The user's t364 traces: the finisher no longer turns, but in KH2's baked swings the model trails the screen's centre
  and jumps back to it at the swing's end, while KH1's own swings look right. Swing totals: KH2's 230 / 182 / 221 / 330
  units, KH1's 92 / 89 / 95 / 186.
- From the decomp (Steam), correcting stage 363: KH1 does take travel from the motion. A request entry's bit 15
  (Sora's ground combo, 0xC8..0xCB; not his air combo) becomes `0x4000000` (`0x1402A01E0`) and the actor's `+0x160`
  bit 4 (`0x14029FC40`). The pose pass (`0x140391C70`) writes H10's translation to the instance's `+0x100` and the
  world position plus it to `+0xF0`; the actor copies `+0xF0` to `+0x50` (`0x1403948D0`) and keeps this frame's step
  in `+0x200` and the travel so far in `+0x210` (end of `0x14029F0A0`). The move function (`0x14029A3C0`, which calls
  the ground step `0x1402B1850`) adds `+0x200` to the displacement; the draw pulls the placement back by `+0x210`, put
  back and cleared at the motion's end (`0x14029FC40`). KH1's own swings carry the same travel in H2, which the body
  hangs from (98..101: H10 z 100 / 120 / 100 / 190), so the model stays on him.
- Ours kept the template's H10 (KH1's travel) with the travel taken out of the body, and `ground_step` fed KH2's
  travel too: he moved by both, and the model was pulled back by KH1's travel with nothing to cancel it.
- `kh1_data::bake::baked` takes `travel` (per frame, model space): the template's H10 translation channels are
  replaced by it (linear keys, reduced). `Baker::poses(.., travel)` puts KH2's root travel along the ground (from
  frame 0, times the retarget's hip ratio, `Retarget::hip_ratio`) back on the whole pose and returns it.
  `kh2slots::prepare` bakes it for the ground combo when the request has bit 15 (`travels`); `Move.travel`,
  `moves_itself(actor)`.
- `shared::physics::ground_step`: for a move that moves itself, KH2's swing travel isn't fed (KH1's step and its own
  root travel move him). The t363 / t364 feed stays for unbaked swings. Air combo unchanged.
- Travel scales with the hip ratio (no foot slide on KH1 Sora's shorter legs), so lunges are shorter than KH2's by
  that ratio and Sora's world scale.

### 2026-10-22: Stage 364 (testing branch: the finisher no longer turns toward KH2's travel)
- The user's t363 traces: KH2's finisher (154) turned Sora mid-swing (0.196 rad a frame, twice) and ran him off the
  screen's centre, then snapped back; swings 151 / 152 unchanged; the stick plays no part (the user).
- From the decomp: KH1's player control (`0x1402A3300`) runs the state's input filter (`0x1402A7B30` -> state 0's
  `0x1402A5F10`): with the stick let go it takes the move vector `+0x20` as the input; the ground pre-step
  (`0x1402CC1A0`) then sets the desired facing `+0x150` from it (`0x140297FB0`, atan2) and KH1 turns his facing
  toward that at the stats' rate (`+0xD0`, π/16 in an attack). t363 left KH2's travel direction in `+0x20`, sideways
  in the finisher, so he turned toward it.
- `ground_step`: KH1's move vector and speed (`+0x20..+0x2C`) are saved before KH2's travel is written and put back
  after KH1's step, so KH1's step still moves him by KH2's travel and nothing else sees it.
- Not explained yet: the ~26 units a frame while he turned and the 28-unit snap back (a new trace will show whether
  they went with the turn).

### 2026-10-22: Stage 363 (testing branch: KH2's swing travel through KH1's own move fields)
- The user: KH2's ground combo travel (physics KH2) into KH1's systems, the goal being every KH2 system inside KH1's
  while keeping KH2's feel. From the decomp: KH1's ground state step (`0x1402B1850`, Steam) moves him by his move
  vector (`+0x20`, laid on the ground normal `+0xC0`) times his speed (`+0x2C`) times the frame; the motion's travel
  helper (H10, the instance's `+0x100`) isn't read by it or by the combo, so baking travel into the motion wouldn't
  move him.
- `shared::physics::ground_step`: in KH1's ground swing (slots 98..101, action flag 4) at physics KH2, KH2's travel
  for that swing (the same clip root, stretch and facing as before) is written as KH1's move vector (flat) and speed
  (units a 60 Hz frame) before KH1's step runs, and KH1's step moves him (on slopes along the ground, as KH1 does).
  It used to overwrite the step's result afterwards.

### 2026-10-22: Stage 362 (testing branch: the baked swings hold the Keyblade teeth up)
- The user's t353 test: the baked combo slots play (log "KH1 plays ours", trace `draw_route` baked) and everything
  translates except the Keyblade, held the wrong way round (teeth down), as expected from the raw bake.
- `Baker::poses` turns the grip and everything under it half round about the blade (`turn_grip`, split out of
  `keyblade_teeth_up`) before the keychain, as the drawn path does, while `keyblade_turn` is on. Every weapon gets it
  in the bake, the dream weapons and the wooden sword too (the user: they show the same issue; option A).

### 2026-10-22: Stage 361 (testing branch: the debug menu's equip crash)
- The user: the game crashes equipping another weapon from the debug menu. The log: `CRASH 0xc0000005 at exe+286737`,
  inside KH1's weapon change (Steam `0x140286720`) at its first use of the id (the item record lookup
  `0x14028F970`, which takes all of `ecx`); `rcx` was `0x2c296451`: the id `0x51` in the low byte, the rest left over
  by the byte argument the t360 call declared. The id is passed as a whole `u32` now.

### 2026-10-22: Stage 360 (testing branch: the debug menu equips Sora's weapon)
- The user: stuck on the Dream Sword after the debug menu gave it ("game won't let me unequip dream weapons"; the log
  shows weapon `0x52` from the Equipment page on): KH1's Equipment page refuses Sora's weapon row while he holds
  `0x52..0x55` (`0x1402E6C60` / Steam `0x1402E90A0`, a four-id list read from the exe). Then: "if its possible to let
  sora equip weapons via the debug menu lets do that".
- **Weapons page**: Cross now equips the weapon on the row (one is given first when Sora owns none) through KH1's own
  weapon change `0x1402845A0` (Steam `0x140286720`; `steam_map.in` gets the pair), which writes his weapon byte and
  loads and makes the weapon object, then the Status copy's refresh `0x1402E5A80`. Left / right still change the count.
  The equipped one is marked. The live stats block follows at KH1's next rebuild (the pause menu's exit, an area).

### 2026-10-22: Stage 359 (testing branch: the debug menu's Weapons and Abilities pages)
- The user: under the Stats page, a weapon and an ability page "so I can add or remove each weapon and ability".
- **Weapons**: every Sora weapon (item kind 2 in KH1's item table) with its inventory count (`save + 0x499 + id`,
  `docs/kh1-equipment.md` section 2; equipping changes no count): left / right add or take one (0..99, never below
  the copies equipped across the members), Cross gives one or takes every free copy. The equipped one is marked.
- **Abilities**: Sora's KH1 abilities (1, 2, 5..28, 53..65: not the party's 29..52, not Glide / Superglide) and ours
  (`growth::MENU`, not KH2's Glide: the Glide style swaps those), each with its copies in his lists and how many are
  on (`save + 0x44`, 48 bytes, bit 7 = off; High Jump and Mermaid Kick in the shared `+0x599`, 10): right adds a copy
  (off, as KH1 learns), left takes one (an off copy first), Cross adds one or takes them all; a full list refuses.
- What the page adds or takes out is left alone for the session by the mod's upkeep (`menu::Growth::update`): KH2's
  abilities given by level / boss / namesake aren't given back once taken out, an added Aerial Dodge isn't held back
  until Genie Jafar, an added Zantetsuken stays, and at `abilities = test / all` added ones of ours aren't stripped
  (`debugmenu::ability_forced`).
- Long pages scroll (16 rows around the cursor, the title shows the place).

### 2026-10-22: Stage 358 (testing branch: the debug menu's stats reach the save too; Level)
- The user: the Status page's Level is `exe+2DE9364` (Steam), which the decomp shows is Sora's save record's `+4`
  (the save block `0x142DE9360` Steam, `0x16C00` bytes, written to and read from the save file), read by the
  Status page itself (Steam `0x140312410`, `0x14031272D movzx eax, [r13+4]`, r13 = save + member x 0x74; the
  page's EXP is the record's `+0x40`, its HP / MP / Strength / Defense / AP the Status copy's). Then: "touch saves
  as well, and any equivalent values let our debug menu adjust with the same settings".
- `debugmenu`'s Stats page has Level (1..100, save only) first, and every stat is written to Sora's save record as
  well as the world block and the Status copy, by edits and by locks (every frame). The record holds HP (`+5`) and
  MP (`+7`) as they are, and base max HP (`+6`), max MP (`+8`), AP (`+9`), Strength (`+0xA`), Defense (`+0xB`)
  before equipment (KH1's stats function adds the weapon's and accessories' and caps Strength, Defense, AP at 100):
  for those the record gets the value less the equipment's bonus, read from KH1's own stats function (unhooked,
  `menu::kh1_stats`) less the record's byte, so KH1's next rebuild gives the value back. Bytes, 0..255.

### 2026-10-22: Stage 357 (testing branch: the debug menu follows KH1's confirm button)
- The user (Circle to confirm): the debug menu's confirm and back buttons follow the game's setting. KH1 keeps them
  as pad masks, confirm at `0x140546390` (Steam `0x1405456A0`, the Config row's `CONFIRM_MASK_RVA`) and cancel at
  `+4`, both set by `0x1402F6700` (Steam) from the setting: Circle `0x2000` / Cross `0x4000`. `debugmenu::buttons`
  reads them each frame (Cross / Circle if they hold anything else); the hints show X or O to match.

### 2026-10-22: Stage 356 (testing branch: the debug menu's crash as it opened)
- The user's t355 log: the game crashed the frame the menu opened, in KH1's command-list getter (Steam
  `0x1400FD238`, reading `0x84`: the renderer's command buffer `+0x5988` null). The menu was drawn from KH1's frame
  end (`0x140109B40` Steam), and by then the field loop has already called the end of the frame's drawing (Steam
  `0x1400FBBE0`, which closes the frame's list with `0x1400F7160`): nothing is recording.
- The menu is drawn from a hook on that function's start instead (Steam `0x1400FBBE0`, Epic `0x1400FBD80` from the
  table's neighbours, prologue `40 56 57 41 56 48 83 EC 40`; the field loop `0x140170D50` calls it after the HUD's
  tasks), once a frame (KH1's frame end calls it again). The frame end keeps the menu's input and the stat locks.
- `fx12::begin` gives up when the renderer has neither a cached list (`+0x5990`) nor a command buffer (`+0x5988`),
  so no draw of ours can reach the getter with nothing open.

### 2026-10-22: Stage 355 (testing branch: t354's build error)
- The user's build of t354: `takeover::trace_frame`'s `trace::Row` lacked the new `cats` field (E0063). It is set to 0
  there, as `trace::frame` fills it.

### 2026-10-22: Stage 354 (testing branch: the debug menu: Trace Recorder with categories, Stats with locks)
- The user: a debug menu opened with a button combination, drawn over the game like the command menu (the game
  runs on), a grey box with some opacity, white text, the selected row yellow; turned on in the Config menu; pages
  that replace the main one, Back out of them, Exit on the main one; Sora not moved by the buttons while it's open.
  `debugmenu.rs`.
- **Opening**: the Config menu's new Debug Menu row (Off / On, ini `debug_menu`, off by default). L1 + R1 + Triangle
  opens it and closes it; closed that way it opens again on the page it was left on; Exit (or Circle on the main
  page) closes it and the next open starts on the main page. While L1 + R1 are held with the menu on, KH1 doesn't
  see Triangle.
- **Input**: `probe::pad` hands the merged pad to `debugmenu::pad`; while the menu is open (and after it closes,
  until every button is let go) KH1's records get no buttons and centred sticks, and KH2's `RAW_BUTTONS` /
  `RAW_STICKS` read nothing. The menu reads the pad as pressed: D-pad up / down, left / right (repeat when held),
  Cross picks, Circle goes back. (Controllers only: the keyboard path isn't covered.)
- **Drawing**: a hook on KH1's frame end (`0x140109CE0 (renderer)`, Steam `0x140109B40`, prologue
  `48 8B C4 57 48 81 EC 90 00 00 00`, from the Steam decomp and exe: `0x1400DAFF0` -> `0x140109B20` -> it, before the
  resolve, full-screen copy and present) draws it over everything that frame (HUD included) with
  `fx12::draw_overlay`: screen-space quads (view identity, a pixels-to-clip projection), no depth test, alpha
  blended; text from a font atlas (DejaVu Sans Mono Bold, 14 x 28 cells, `debugfont.rs`, run-length coded) kept with
  the model textures. `fx12` also uploads effect textures handed over after the renderer came up (the overlay can
  bring it up first: `late_textures`).
- **Trace Recorder page**: Trace (Off / On / Always), Seconds (1..120, steps freely), Sound Log (On / Off), and
  Categories: six groups (KH1 Actor, KH2 Controller, Pose, Camera, Input & Config, Events) whose rows turn all of
  the group on or off with left / right and open with Cross to set each of the 13 items (`trace::ITEMS`,
  `GROUPS`). Every change is written to the ini (`trace`, `trace_seconds`, `sound_log`, new `trace_*` keys, all on
  by default) and is live at once (`trace::set_*`; `trace::refresh` takes the ini's again when it reloads).
- **Trace categories**: a category that is off isn't gathered (the camera's `record`, the trace's draw / feet
  reads, the feet hook, KH2's side and Form in `takeover::trace_frame`, the hit-effect lines' formatting) and its
  columns leave the CSV (the header is built from the categories; each row keeps the categories it was made
  with, so a dump across a change writes empty cells where a row lacks a column). Log lines, hit-effect lines
  and the sound log's lines are separate categories of the events column (`log_as`, `trace::note_as`).
- **Sound log**: its hooks go in whatever the ini says, and write only while it's on (`trace::sound_log`), so it
  switches in game.
- **Stats page**: Sora's HP, Max HP, MP, Max MP, Strength, Defense, AP (left / right change, Cross locks). From the
  Steam decomp of KH1's stats function (`0x1402A5130`, Epic `0x1402A2FA0`): the stats block's `+0x3C` HP (at most
  max HP), `+0x40` max HP, `+0x44` MP, `+0x48` max MP, `+0x4C` Strength, `+0x50` Defense, `+0xE4` AP, `+0xC8` the
  save record it came from; KH1 fills the Status menu's copies at `0x142E94BC0 + member * 0x100` (Steam
  `0x142E941E0`, `0x9E0` below as its neighbours in the table) and Sora's world block (actor `+0x6C`) from the save
  and copies his HP / MP back into the save after. Changes go to the world block and the Status menu copy, never
  the save; locked stats are written to both every frame, after KH1's stats function fills a block for Sora
  (`menu::stats` -> `debugmenu::lock_stats`) and after Valor's stat step (`valorstats::step`). HP and MP stay within
  their maximums; a locked HP above 0 clears the block's dying flag (bit 4 of `+0`, as KH1's HP change does). The
  locks are not kept in the ini.

### 2026-10-22: Stage 353 (testing branch: KH2's slots added where KH1 binds Sora's motion sets)
- The user's t352 log: the set on Sora's actor in Traverse Town is the world's (about 88 slots, the combo at
  61-67), not his own 118, and its request table has 0xFFF for what it lacks. From the decomp (the user: look
  first): his own set is bound at the actor's creation by `0x140287E40` (Epic `0x140285CC0`), a world's on
  entering it by `0x14029E850` -> `0x1402886F0` (Epic `0x140286570`, from the neighbours, unchecked: the prologue
  test keeps it unhooked if wrong), his own saved at `+0x1E0`. `docs/kh1-anim-slots.md` §2.
- Both binds hooked (`kh2slots::install`): after KH1 binds a set onto Sora's actor (its model 293 bones), the
  slot and request handles become ours for that set, made once per set: its own slots (counted as the words before
  its first motion: no end marker in memory), ours right after; its request table with the requests our moves stand
  in for pointed at ours by style, only where the set's own entry holds the KH1 motion ours stands in for (by
  signature). Each set logged ("Sora's set ...: N slots of its own, ours at ...").
- `frame` no longer swaps or checks against the file: it extends `+0x1D0` and `+0x1E0` if a bind came before our
  motions were built, sets the redirects by style in every set, and ticks. Our moves are found by their motion's
  address (`current`), so `kh1_time`, `tick` and the trace work whatever slot number the set gives them.
- Removed: `kh1_own`, `describe_slots`, the per-frame table copy and the 118-slot assumption.

### 2026-10-22: Stage 352 (testing branch: KH1's slot table taken as its 118 entries; how KH1 lays it out, logged)
- The user's t351 log: KH1's slot table on Sora's actor has empty entries 80-88, 91, 104-107, 116, and no end marker
  after entry 117 (entry 118 = 02000000, not a handle; the "135 entries" counted data past the table). The decomp
  (`0x140287E40`, `0x1401D6230`): the actor's `+0x1D4` is the bound file's MMTN `+0x30`, its list at MMTN `+0x40`.
- `kh1_own` no longer asks for an end marker: the table is taken as the file's 118 entries (ours keeps its own end).
  Once per table it logs (`describe_slots`) which entries are empty, which hold another file slot's motion (by
  `pose::file_slot_of_motion`, the signature `slot_of` uses), which hold something not in Sora's file, and the
  words after entry 117. The request table test stays.

### 2026-10-22: Stage 351 (testing branch: KH2's slots go on Sora with KH1's empty slot entries)
- The user's t350 log: Sora's actor refused because KH1's slot table (in memory) has entry 80 of 118 empty, though
  the file has a motion there (`kh1_own`'s first test). The request table wasn't reached.
- `kh1_own` no longer requires every one of KH1's 118 entries: empty ones are copied as they are (logged once, which
  ones); only an all-empty table is refused. The slot table length and request table tests stay, with their reasons.
- Ours keeps KH1's own entries up to date each frame from KH1's table on the same actor (`Placed::kh1_slots`), so an
  entry KH1 fills after ours was made isn't left empty in ours.

### 2026-10-22: Stage 350 (testing branch: why KH2's slots aren't put on Sora, logged)
- The user's t349 run: all seven KH2 motions baked (KH1's FK within 0.22-0.30 units of the poses), but no slot of
  ours ever played: `kh2slots::frame` found Sora's actor "showing another motion set" every frame (logged 5 times,
  never "on Sora's actor"), so KH1 played its own 98-104 with KH2's swings drawn over them (trace `draw_route`
  follow, `Swing`). Tier A never ran in game either.
- `kh1_own` now says which test fails: the tables unreadable (handles and addresses), an empty slot table entry,
  the slot table's length if not 118 (with entry 118), or the request table's entries that differ from the file
  (count, first 12 with file and memory values). Each new reason is logged ("not taken (...)"), 40 at most.

### 2026-10-22: Stage 349 (testing branch: KH2's slot motions baked into KH1 motions, tier B)
- The user: bake KH2's animations into KH1 Sora's motions (`docs/kh1-anim-slots.md` §1.6, §3.1), a raw bake first
  (none of our corrections: Keyblade teeth-up, `flip_weapons`, our feet, `held`), behind an ini switch, then test
  in game. The Keyblade is part of KH1 Sora's skeleton (bones 280-292, its mesh skinned onto them), so KH2's body
  and Keyblade motions bake into the one KH1 motion; Valor's second Keyblade later.
- `kh1_data::bake::baked(template, frames, events, skeleton, poses)`: KH1 model-space poses per frame -> a KH1
  motion on the retimed template. Joint words plain FK (word 0; bit 25 where nothing moves; KH1's skipped leaves
  keep theirs), each bone's local as Euler X->Y->Z (continuous across frames) and translation, bind values left
  out, constants for held values, linear keys reduced within 0.001 rad / 0.02 units; the template's helpers and
  helper channels kept (H10's travel moves him as in tier A); KH1's FK from the channels checked against the
  poses (logged as "within N units").
- `pose::Baker` (loader thread): KH1's skeleton from `xa_ex_0010.mdls`, the retarget with the battle / field idles
  as rest, `secondary`; `poses` draws each frame as a tier A slot does (KH2's body and Keyblade, root travel out,
  retargeted, KH2's hair, hood and keychain).
- `kh2slots::prepare(mset, kh2, bake)`: with `kh1_bake = on` (new ini key, default on) each slot's motion is the
  baked one (`Move::baked`), else the retimed template as before; a failed bake falls back to the template.
- `pose::follow`: a baked slot draws nothing (KH1's own pass shows it); KH2's pose at KH1's time is still noted
  for KH2's effects on his bones (`kh2fx`, `reflect`). Trace `draw_route` "baked" with its motion and frame.
- Legs are FK in this first bake: no KH1 foot lift on slopes during these moves (tier A had none there either).
- Nothing tested in game; nothing compiled here (no crates in this environment).

### 2026-10-22: Stage 348 (testing branch: t347's trace row builds again)
- t347 didn't build: `takeover`'s `trace::Row` initializer lacked the new `feet` field (filled by `trace::frame`
  from `pose::take_feet`, as `draw` and `cam` are). Given its default there.

### 2026-10-22: Stage 347 (testing branch: the trace shows what KH1's own skeleton pass does to his feet)
- The user, before the tier B bake (`docs/kh1-anim-slots.md` §3.1): test in game whether KH1 bends Sora's legs
  onto a slope, since a plain-FK baked slot turns KH1's two-bone leg IK, and with it its ground lift, off.
  From the decomp (§1.6): `instance + 0x14` bits 0-1 turn the ground lift on; bit `0x200000` is set when a
  foot target was lifted onto the floor.
- `pose::note_feet`, straight after KH1's pass (`0x140391E20` or `0x140392710`) and before anything of ours,
  for Sora's own instance while the trace runs: that word and the ankles' heights (bones 19 and 45).
- Trace columns after `kh2cam_lock_dist`: `ik_ground_on` (bits 0-1), `ik_lifted`, `ik_flags`, `foot_r_up`
  and `foot_l_up` (each ankle above `floor_up`, the floor under his centre), `pose_path` (which pass ran since
  the last row, with counts; empty columns when KH1 didn't pose him).
- The test: Insert to "off (KH1's own animation)", stand side-on on a slope or stairs, Select + Triangle.

### 2026-10-22: Stage 346 (testing branch: KH2's pose and effects once per game frame, the game's own frame delta)
- The user: KH2's side should run on the game's own delta (30 fps base, 60 fps and uncapped on PC), and the
  hook is heavy on low-spec hardware (at 30 fps every KH2 step ran twice, at a hitch up to four times).
  From the Ghidra decomp of the Steam exe (`FUN_140154740`, `FUN_140154700`, the pacing thread at ~line
  255080; project note `kh2-frame-delta.md`): the delta (`0x140717480`) is always a whole number of 60 Hz
  units, one update of delta 2.0 at the 30 fps base, no fractional delta at higher or uncapped rates (those
  only add presents between logic callbacks, ~59.94 Hz at most).
- Kept: the controller, KH1's mover, hits, guard windows and sounds still step in whole 60 Hz frames
  (`takeover::steps`), so no window is skipped and KH2's moves take the same time at either frame rate.
- **Pose once per game frame** (`kh2-rig` `RigAnim::update_deferred` / `settle`, `takeover`): a step before
  the frame's last keeps the animation's track and blend clock moving but doesn't evaluate the clip, blend or
  weapons; the last one (or `settle` after an early end of the steps) does. A step with a hit or guard window
  open still evaluates (hits read the Keyblade's place). `update` is `advance` + `render`, the same as before.
- **Effects take the frame's delta** (`kh2fx::tick(motion, delta)`, `takeover::physics`): one tick of `steps()`
  60 Hz frames instead of that many ticks of 1.0 (`kh2_fx::Effects::tick` adds the delta up as the game's
  effect clock does); KH2's frames tick after the steps, with the steps run.
- Nothing tested in game; the load at 30 fps should drop (one pose and one effect pass a frame instead of two).

### 2026-10-22: Stage 345 (testing branch: KH2's spells hit with the KH1 spell's own hit effect)
- The user: KH2's moves' hits only ever show the plain Keyblade effect. From the user's Ghidra export of the
  Steam exe (project notes `kh1-hit-effects.md`): KH1's hit spark comes from the attack object's effect list
  (`+8`), picked by the hit kind and the attack data's `+0x28` group; a swing's list is the equipped Keyblade's
  (`new_attack` on Sora), a KH1 spell shot's the spell's own. The spells' lists live in KH1's **magic slots**
  (`0x2D23580`, 0x38 each, one per KH1 spell: `+0` level, `+8` the loaded `.mag` file; the list at
  `file + [file + 8]`); the t341 run showed them staying loaded with no KH1 cast.
- `land_hit` for a spell (KH1 spell data): the KH1 spell's list from its slot goes in the attack object's `+8`
  (`pose::handle`, a pointer to a game handle) before KH1's contact: Fire's, Blizzard's and Stop's hit sparks
  (Thunder and Gravity have none, as in KH1). Slot not loaded: the Keyblade's, as before. Logged ("KH2 spell
  hit ...", 40 lines) and in the `hitfx:` trace note.
- `probe`: the t340-t344 investigation scans (actor / object table / data searches) are taken out again; the
  per-hit "hit effect:" log and effect-list dumps stay.

### 2026-10-22: Stage 344 (testing branch: the object-table scan takes any file that holds the list)
- The user's t343 run: the spells' list handles turn up only in passing places (two "last one" words at RVA
  `0x2D223E4` / `0x2DB4718`, the running effects' pool from `0x2D678B0` at 0x3C each, the attack objects from
  `0x2DB4718` at 0xC0 each), nothing that keeps them; KH1's actor loader (`0x140288460`) has other file
  layouts than the one the t342 scan checked (`+8` .. `+0xC` for some kinds, `0x140287E40`).
- `scan_object_tables` now reports any entry whose file (handle `+8` or slot buffer) holds the list within
  1 MB, with the list's offset in the file and the file's first 8 words (its section table).

### 2026-10-22: Stage 343 (testing branch: what holds the KH1 spells' effect lists)
- The user's t342 run (all of KH1's magic): Fire, Blizzard, Thunder, Gravity and Stop hit with their own
  lists (Gravity's has no kind-1 entry, Stop's normal hit is id 2), and no entry of the four object tables
  has a file whose effect section is one of them: they're loaded some other way.
- `probe::scan_refs` (Steam ranges): each such list's handle and address are searched for through the exe's
  data (RVA `0x4D5000 .. 0x2F131E8`) and every place logged as an RVA, to find what keeps them (and the code
  that loads them, from the exe).

### 2026-10-22: Stage 342 (testing branch: which KH1 object entry holds each spell's effect list)
- The user's t341 run: the KH1 spells' effect lists stay loaded and unchanged after Magic went to KH2 and a
  room change (same world), with no KH1 cast. They sit in a load arena (packed by load order), so they're
  found the way KH1 finds them: object entry -> model file -> effect section.
- `probe::scan_object_tables` (Steam only: the user's Epic exe in `D:\claude\EPIC EXES` is an older Epic
  release, contact at `0x2AF890` not `0x2B1ED0`, so the mod's Epic addresses for these tables aren't known):
  when a hit uses a list that isn't the Keyblade's, every entry of KH1's four object tables (types 1, 7, 8,
  12; Steam `0x14296B630`, `0x14298C6F0`, `0x14298C690`, `0x14298C750`) is checked: its file (handle `+8`,
  or its fixed slot buffer at Steam `0x140D2ADA0`) and that file's effect section; the entry whose section
  is the list is logged ("object tables:") with its id, type and slot.

### 2026-10-22: Stage 341 (testing branch: the KH1 spells' effect lists remembered by address)
- The user's t340 run: KH1's Fire, Blizzard and Thunder hit with lists at `0x...6c2b80`, `0x...6de500`,
  `0x...702d00`, the very addresses of the run before (a game restart apart), and no live actor in KH1's actor
  list has them as `+0x68` (the spell shots aren't in it). They're remembered by address now (logged with
  their RVA), so a KH2 spell's hit shows whether each list is still there, unchanged.

### 2026-10-22: Stage 340 (testing branch: where KH1's spells keep their effect lists)
- The user's t339 run: all six Keyblades share one weapon effect source, reloaded on equip (each list differs:
  Diamond Dust's sparks are ids 5..7, the others 0..2). From the Steam exe (project notes
  `kh1-hit-effects.md`): an actor's own effect source is `+0x68`, the effect section of its object's model
  file (section table `+0x10 .. +0x14`), set when it's made; KH1's spell shots put the spell actor's `+0x68` in
  their attack objects' `+8`. A KH2 spell could do the same with the KH1 spell object's file, if it's loaded.
- Log (`probe`): a hit with a list that isn't the Keyblade's finds the live actor whose `+0x68` it is, and logs
  its object entry (id, type, slot, file) and the file's effect section ("spell source:"); every KH2 spell hit
  logs those remembered spells' entries, files and lists as they are now ("spell sources (...)"), to see if
  they stay loaded without a KH1 cast (a room change).

### 2026-10-22: Stage 339 (testing branch: the hit-effect log per Keyblade)
- The user's t338 run (six Keyblades, KH1 Fire / Blizzard / Thunder, KH2 Fire / Blizzard / Thunder): KH1's
  spells hit with their own effect lists (Fire spark id 3, Blizzard 2, Thunder none of kind 1); KH2's spells
  with the Keyblade's (the bug); all six Keyblades showed one source address, so only Kingdom Key's list
  was logged. The log is now keyed by the list's contents too (a hash) and names the equipped weapon (save
  entry `+0x36`), to see each Keyblade's list.

### 2026-10-22: Stage 338 (testing branch: which KH1 hit effect each hit gets, logged)
- From the Steam exe (project notes `kh1-hit-effects.md`): KH1's contact `0x1402B1ED0` starts the hit
  effect with `0x1402B2190 (attack, kind, at)`: the source is the attack's weapon's effect list
  (`attack+8`, made from `actor+0x14C` -> `+0x18`: Sora's equipped Keyblade, so each Keyblade has its own
  sparks; a KH1 spell's own actor for its attacks), the entry the first whose kind mask holds the hit
  kind (1; 4 against a guard, looked up as 2; 0x40 when the record's `+0xC` has 0x20) and whose group
  mask holds `1 << (attack data +0x28)`; started at the hit point with `0x1402AC4C0`. Byte `+0xA8` is
  only the hit sound (bit `0x400000`). The user: Keyblades' hit effects do look different.
- `probe::hit` (any attack on anyone but Sora) works out that lookup: every hit's source, attack data
  group, kind and the spark id KH1 picks go to the trace ("hit effect:"); each new (source, group) and
  each new source's whole effect list (each entry's kind / group masks) to the log, to see which ids are
  hit sparks, Keyblade by Keyblade and for KH1's own spells.

### 2026-10-22: Stage 337 (testing branch: `hit_effect_test`, KH1's effects started on a KH2 hit)
- The user's t336 `hit_fx_test` runs (KH2 air combo, 12 values each): byte 0 of `+0xA8` is the hit
  sound (KH1 sound 11024 + the value), bytes 1 and 2 changed nothing heard or seen; no value of any
  byte changed the hit's look. KH1's contact plays one plain spark; a different look has to be
  started by us at the hit (the user's plan: per custom move, a KH1 effect or their own).
- Test (ini, off by default, read live): `hit_effect_test = ids` (ranges allowed, `0-20,30`) starts
  the next id, in turn, on each landed KH2 hit through `0x1402ABB60 (source, id, actor, 0x30)` from
  Sora's effect source (`+0x14C` -> `+0x18`), as `swing_start` does with a row's effect; on the foe
  hit (`hit_effect_on = target`) or on Sora (`= sora`). The id, actor, source and the call's return
  are logged and in the `hitfx:` trace note.

### 2026-10-22: Stage 336 (testing branch: the ini is read again whenever it is saved)
- The user: "can you make it so that the ini can be changed while the game is running?"
  `settings::get` checks the ini's modified time at most every half second and parses it again
  when it changed (logged: "settings reloaded"); the new settings replace the old (the old are
  leaked, as callers may still hold them). Settings only used at start (hooks put in, the
  abilities given, the trace mode's setup) still need a restart.

### 2026-10-22: Stage 335 (testing branch: `hit_fx_test`, which of KH1's hit effect bytes picks the hit effect)
- The user's t334 traces: a KH2 spell's attack object lacks bit `0x400000` (`000b5800`), swings
  have it (`00415809`, the contact adds `0x100000`). The swings' `+0xA8` bytes come only from the
  borrowed KH1 row: `[00, 05, 06]` on the swings, `[03, 05, 06]` on the air finisher (row 206). The
  user: the finisher's hit sound changed, its effect didn't. So is byte 0 the effect at all?
- Test (ini, off by default): `hit_fx_test = 0,1,2,...` writes the next value, in turn, into byte
  `hit_fx_byte` (0..2) of each landed KH2 hit's `+0xA8` bytes (the rest from the row), with bit
  `0x400000` set, spells too; logged and in the `hitfx:` trace note.

### 2026-10-22: Stage 334 (testing branch: the attack object's flag word in the hit-effect trace)
- The user's t333 trace (KH2 Blizzard, ATKP 248 sub 4, on an enemy): the spell hit was landed with
  KH1's Blizzard level 3 data (`0x6158`); the object's `+0xA8` held that data's own bytes
  `[03, 05, 06]` before and after the contact, yet the plain Keyblade hit effect played. Spells
  don't get bit `0x400000` on the object's flag word (swings do, as the Attack command's row does);
  is that bit what makes the contact use `+0xA8`? `hitfx:` now also shows the flag word before and
  after the contact (and whether `0x400000` is set), to compare a spell hit with a swing hit.

### 2026-10-22: Stage 333 (testing branch: trace notes for the hit effects of KH2's moves)
- The user: KH2's custom moves' hits only ever spawn the plain Keyblade swing effect. Every KH2 hit
  is landed (`land_hit`) through one of Sora's four KH1 rows (ground / air, swing / finisher), and
  that row's effect / sound bytes go to the attack object's `+0xA8`; the KH2 ATKP's own effect
  fields are not passed on. Trace notes (events column) to confirm before changing it:
  - `hitfx:` on every landed hit: the KH2 side (swing key or spell, ATKP id / sub, kind, element,
    reaction, effect_on_hit, damage_effect, random_effect, hit_sound, flags), the KH1 row picked
    (index, motion, data offset, fx bytes, trail / effect / voice), the attack object's
    `+0xA8..+0xAA` before and after KH1's contact, KH1's attack data words, target and damage.
  - `swing start fx:` the KH1 combo motion whose trail / effect / voice a KH2 swing starts with.

### 2026-10-22: Stage 332 (testing branch: a KH1 air swing out of any KH2 air state gets KH1's hop, not KH2's rise)
- The user (trace f8556, Explosion, then a jump and a swing on the jump's first frames): the swing
  sent him up. The swing came out of `combat::command`'s air command (KH2 had him in its Jump /
  Air), which passed `keep_rise = true` to `kh1_air_attack_started`, so KH2's jump rise (269 left)
  went on under the swing: 20.5/frame up, 184 up at the swing's 10th frame, 258 at its 17th. Stage
  313 had done this only for the swing out of KH2's jump arc on KH1's air states.
- The user's call, "grounded attacks only": no KH2 air state keeps its rise now (Jump, Air rising,
  Aerial Dodge, an air swing or cast): `keep_rise` is false at both callers; the faster sideways
  speed is still kept. Falling swings are as before (KH2's rise was 0 there).

### 2026-10-22: Stage 331 (testing branch: KH2's glide 1:1 with the Steam exe: the stored launch speed, no stick-scaled glide, no glide back to the hover)
- The user: "neutral flight doesn't store aerial dodges momentum, in kh2 ... if you aerial dodge go
  into neutral flight, neutral flight will store the speed from aerial dodge, so when you glide you
  go to max speed, or if you glide then let go you get a bunch of speed while your falling"; "it
  should be 1:1 so change everything that is currently different". (The rust rewrite's exe notes
  are the Epic build's; these are the Steam exe's, `KINGDOM HEARTS II FINAL MIX.exe`, found from the
  RTTI of `ACTION_GLIDE` / `ACTION_GLIDEHOVER`, vtables `0x1405CDBF0` / `0x1405CDB98`.)
- What the exe does: Sora's movement data keeps his velocity (`+0x20`) and a stored launch velocity
  (`+0xE0`, flag `+0xEC`). The jump's and the Aerial Dodge's launch (`0x1403CBB00`, the dodge from
  `0x1403CB9F0`: stick direction x the dodge's speed, 0 neutral) writes it and sets the flag; the
  air actions' shared enter (`0x1403CBD30`, jump, glide, hover) copies it back into his velocity
  while the flag is set; the hover's step (`0x140423650`) multiplies his velocity by 0.9
  (`0x1405AAA40`) and leaves `+0xE0` alone; the stick takes the hover into the glide
  (`0x140423840`), whose enter puts the launch speed back; the glide's step (`0x140423920`) is
  `(1 - a) * glide_speed + a * |v|` along his facing, no stick tilt in it; the glide's end into the
  fall (`0x1403CBC30`) clears the flag and keeps his velocity. The start picks the hover only with
  the stick neutral (`0x1403A8369`: action 0x11, else the glide, 0x12); the glide never goes back
  to the hover.
- `character-core`: `Character::launch_vel` (set at the jump's take-off and the Aerial Dodge, put back
  on `try_glide` and on the hover's turn into the glide, cleared at the glide's end and on landing);
  the glide's speed no longer follows the stick (stage 173 out), and a glide let go no longer drops
  into the hover once slow (stage 178's ours, `GLIDE_HOVER_SPEED`, out). Tests:
  `glide_in_place_hovers` (the glide flies on with the stick let go), new `hover_keeps_the_dodge_speed`.
- `takeover`: KH2's stored launch speed is kept when KH2 gives him back in the air (`LAUNCH`: from
  the glide's borrow, or a borrow that launched him), and the glide's borrow later in the same air
  time starts from it (an Aerial Dodge, KH1's fall, then Square: the dodge's speed comes back);
  without one (KH1's own jump) his speed at the borrow stands in for it. Cleared on the ground.

### 2026-10-22: Stage 330 (testing branch: the trace's camera columns compile)
- Stage 329 left `cam` out of the trace row `takeover::trace_frame` builds (E0063); it is
  `Default::default()` there, filled by `trace::frame` like `draw`.

### 2026-10-22: Stage 329 (testing branch: Sora's camera in the trace)
- The user: "add soras camera movements to the trace ... when I describe whats happening you can
  better understand and compare with the kh2 decomps camera and what we currently have"; the
  trace is the Select + Triangle recording.
- `camera.rs` notes each call of KH1's follow / lock-on / movement-state camera mode (`Seen`, read
  by `trace::frame` through `camera::seen`): who placed it (ours or KH1's, and why not ours:
  `kh1_chosen`, `event`, `kh1_only_state`, `scene`, or what `run` declined: `subject_not_sora`,
  `bad_feet`, `degenerate_camera`...), which entry (`follow`, `lock`, `follow<-stateN` for a
  movement-state camera ours took, `stateN` for one KH1 ran), the mode-entry flags, KH1's eye and
  look point (up = minus y), how far each moved since the call before, the eye's ground distance
  and the eye's and look point's height from Sora's feet, KH1's yaw / pitch / distance / field of
  view / blend, and KH2's camera's yaw, goal yaw, distance, the right and left sticks it read,
  reset, auto and the lock-on target's distance.
- 34 new columns `cam_*` / `kh2cam_*` before `events`. `cam_lag` is the game frames between the
  row and the camera's last run (1 is normal: Sora's physics runs before the camera); more means
  KH1 has the camera in another mode (`cam_mode_now`: event, area, conversation, first person).
- Not in: KH1's public eye / look point (`0x14299C4E0` / `4F0`, what is drawn, after a blend)
  aren't in the Steam address table yet.

### 2026-10-22: Stage 328 (testing branch: sound log names the SFX and voice bank files)
- The user: "we can do both ... if I say I want sora to say this line e.g 0x23 you can just cross examine what I said with the log"; "you can do both" (a sound-effect hook and a clip-to-file map).
- `soundlog.rs` (Steam build only for these): hooks the voice call (`0x1400DC210`, bank + clip no) and the effect create/play (`SslSeCreate`/`SslSePlay`). Voice lines log `voice call bank 93 no 37 = tw_sora.win32.scd clip 37`; effects log bank, no, id and the first bank-list nodes (dumped once, to decode the layout). `docs/kh1-sora-voice-banks.csv` maps Sora's bank/clip to SCD entries.

### 2026-10-22: Stage 327 (testing branch: KH2's glide on Square as the KH2 rewrite has it; KH2's air moves keep their input)
- The user: "it should behave exactly like the kh2 decomp ... in kh2 your allowed to glide out of
  air doge"; "glide should activate from pressing square, not jump". The KH2 rewrite binds the
  glide to the action button (`kh2-bevy` main.rs: "Glide on the action button ... jump still
  Aerial Dodges"; tests `glide_on_the_action_button`, `action_button_mid_dodge_glides`): in the
  air the jump is only the Aerial Dodge; Square glides whenever he is above the glide's start
  height, before or after the dodge or mid-dodge, held to keep it going.
- At Glide KH2 KH2's controller's glide button is Square (`GlideButton::Action`) whatever KH1's
  Superglide or the ini's `glide`. The borrow out of KH1's air (`shared::GLIDE`) is a fresh Square
  (up or down) with KH2's Glide equipped and high enough; its press is KH2's action press on the
  first frame (`ROLL_NOW`), and the Aerial Dodge is there in the borrow if it wasn't used this air
  time (`AIR_MOVES`); used there, it is noted at the hand-back (`shared::mark_air_dodged`) so
  KH1's air doesn't borrow him for a second one.
- The Aerial Dodge hide rule (stage 319: KH1 sees no jump until the dodge is used, then a fresh
  press) is KH1's glide's only: at Glide KH1.
- Stage 323's input fix back in: a KH2 move in the air out of KH1's states (`shared::kh2_move`,
  airborne, KH1 state 0, its "action running" bit up) isn't taken as KH1 holding him still, so the
  glide's held Square and the Aerial Dodge's stick and Square reach KH2's controller.
- Not done (the user: "everything but 4. for now"): hiding KH1's Square in the air at Glide KH2
  (KH1 has no Glide or Superglide there anyway, `glide_swap`).

### 2026-10-22: Stage 326 (testing branch: KH1's sounds in the log)
- The user: "we can do both ... if I say I want sora to say this line e.g 0x23 you can just cross
  examine what I said with the log". New `soundlog.rs` (ini `sound_log`, on by default; `off`
  hooks nothing) hooks KH1's menu sound (`0x140177F60 (id)`), voice group (`0x14028E060`) and
  voice clip (`0x14028E7A0`) and logs each call: `KH1 sound: menu sound 2 (from game +0x...)`,
  `voice group 4 on Sora (...)`, `voice clip 35 (0x23) priority 3 on Sora (...)`. A caller outside
  the game's code is the mod's own call (`kh1voice`, `trace`, `config`, `combat`). The prologues
  are the Steam exe's (read from it): `40 53 48 83 EC 30`, `48 89 5C 24 20`, `48 89 5C 24 08`;
  an Epic build with other bytes logs "not hooked" and goes on. `docs/kh1-sounds.md` is the table
  to fill in from the log; the log is capped at 6000 sound lines a session.

### 2026-10-22: Stage 325 (testing branch: the trace's sound is the menu's confirm)
- The user: stage 322's sound (`0x140177F60 (1)`) is the cursor's move sound; the one wanted is
  the confirm on a menu button. `trace::click` now plays sound 2 (the menu's "open" / on sound,
  `docs/kh1-config-menu.md`) when the trace is written, still 4 (the "can't") when it isn't. If 2 is
  not the confirm, 0 and 3 are the others to try.

### 2026-10-22: Stage 324 (testing branch: stage 323's two fixes taken back out, for now)
- The user: only the Glide ability swap was asked for; the two fixes are to be tested after it.
  Taken back out (as they were at t322): KH2's moves in the air out of KH1's states are again cut
  off from input while KH1's control step stops (`takeover`'s `borrow_air` gone), and at Glide KH2
  KH1's jump and Square are hidden in the air past the rise only (`shared::control`). Still open:
  every borrowed KH2 glide ends on its second frame (the t321 log), and Square on the way up can
  start KH1's own Superglide (less likely now: at Glide KH2 the shared list has no Superglide).

### 2026-10-22: Stage 323 (testing branch: KH2's own Glide ability by the Glide style; KH2's air borrows keep their input)
- The user: "if glide is set to kh2 both glide and super glide need to be removed from the shared
  abilities list and only the original kh2 growth glide should appear again as its own custom
  ability, if its kh1 then kh2s custom glide ability gets removed and it returns to vanilla".
  KH2's own Glide is ours, id 88 (`growth::ours::GLIDE`, `MENU`): shared list, no AP, made from
  KH1's Glide entry, its menu name with its level ("Glide LV2", "Glide MAX", `menu::set_glide_name`),
  `Effect::Kh2Glide`: LV1..LV3 by Sora's level (40, 52), MAX once Chernabog is beaten
  (`growth::kh2_glide_level`). It is never learned on its own: `growth::glide_swap` (each second,
  `menu::glide_style_list`) at Glide KH2 takes KH1's Glide and Superglide out of the shared list
  and puts KH2's Glide in the first one's place (on if either was on; a Glide or Superglide KH1
  gives later is taken out the same way); at Glide KH1 it takes KH2's Glide out and puts KH1's
  Glide back (on as it was) and Superglide too once Chernabog is beaten (on with Glide). Other
  ability modes put KH1's back before taking ours out. "Ability Up" boxes for the glide only for
  KH2's Glide now. With KH1's Superglide gone at Glide KH2, KH2's glide button is the jump.
  Tests `glide_swaps_with_the_style`, `kh2_glide_grows_to_max_at_chernabog`.
- The user: "currently unable to glide or sometimes can?" (t321 log: all 21 glide borrows ended on
  their second frame; trace f3281: KH1's own Superglide). (1) A borrow in the air sets KH1's
  state to 0, which queues KH1's landing and raises its "action running" bit, so KH1's control
  step stops through the whole KH2 move and `takeover` read it as KH1 holding him still: no
  input, so the glide button read as let go (and the Aerial Dodge had no input at all). Now a
  KH2 move in the air out of KH1's states (`shared::kh2_move`, airborne, KH1 state 0, bit 4 up)
  isn't taken as held still. (2) Square on the way up started KH1's own Superglide (the Glide KH2
  hide was past the rise only): KH1's Square bit is hidden all through the air at Glide KH2, its
  jump past the rise.

### 2026-10-22: Stage 322 (testing branch: a sound when a trace is recorded)
- The user: "can you make it so that when I record a trace a sound in game plays the sound when you
  click on something in the pause menu". Select + Triangle's dump (`trace::frame`) now plays KH1's
  menu click (`0x140177F60 (1)`, the sound the Config menu's toggles make) once the CSV is written,
  or the "can't" (4) if it couldn't be (the session's dump limit, a write error). `trace::dump`
  returns whether it wrote the file. The anomaly dumps, written on their own, stay silent.

### 2026-10-22: Stage 321 (testing branch: KH2's glide is KH2's controller's again; the glide off the physics setting)
- The user: "after incorporating kh2 glide as a shared ability it stopped it from being 1:1 with
  kh2's glide" (t320 traces, physics KH1, Glide KH2: KH1's glide state 0x15 on KH1's numbers with
  KH2's motion 205 drawn on it: KH1's +0.4 a frame speed ramp to 16, its 1.0..2.45 saw-tooth fall,
  no turn for a small tilt, and at release KH1's cut to 1.1 sideways with a hop up). Stage 284 had
  made the glide a shared move whose numbers followed the physics setting (`shared::physics`'s
  glide wrappers ran only at physics KH2). The user: option 2, "and not have glide be affected by
  the physics setting in the config menu".
- Glide at KH2 is a KH2 borrow again (`shared::GLIDE`): out of KH1's air, past the rise, the glide
  button (Square with Superglide or `glide = square`; else a fresh jump press once the Aerial
  Dodge isn't there for it: used this fall, or not equipped) with KH2's Glide equipped and above
  its start height (`takeover::kh2_glide_open`) borrows him; KH2's controller gets the press on
  its first frame (`JUMP_NOW` / `ROLL_NOW`, no Aerial Dodge in the borrow) and glides him with
  its own glide: speed, fall, turn, banking (206 / 207), hover (208) and its release into its
  fall, then KH1's fall when that is over (`give_back`). KH1 sees none of its glide inputs in the
  air past the rise at Glide KH2. KH2's controller keeps its Glide ability at Glide KH2 in shared
  states too (it was set to 0 there), so the glide button mid-Aerial Dodge glides straight away.
- Glide at KH1: KH1's own glide state and numbers at either physics setting. The glide wrappers
  on KH1's state 0x15 (`glide_pre` / `glide_step`) and `Numbers::glide` are gone; KH1's Square
  glide (`shared_frame`) only at Glide KH1.
- The glide's numbers never follow the physics setting now (KH2's controller's glide reads only
  KH2's growth table; physics KH1 only changes its fall after the glide is let go).

### 2026-10-22: Stage 320 (testing branch: Superglide on Square, glide mid-dodge, glide-hide logging)
- The user: "you can't superglide out of air dodge"; in vanilla KH1 Glide is the jump button and
  Superglide is Square. t319 had forced `glide_on_square` off, which took Superglide's Square away.
  Now the glide button follows the ability (`takeover::glide_on_square_now`: Superglide equipped, or
  the `glide_on_square` flag): `shared::control`'s hide (`Seen::superglide`), the Square glide in
  `shared_frame`, KH1's glide held while it glides (`gliding`), and the controller's `glide_button`.
- KH2's glide button (`character-core` `try_action_button`, `try_jump`, the AerialDodge action): a
  FRESH press of the glide button glides, also in the middle of the Aerial Dodge ("straight into
  Glide"); a held button never does. `shared_frame`'s Square glide is a fresh press now (a Square
  held through the dodge doesn't glide when KH1 has him back). `style_air_start` (KH1 glide with
  KH2 owning him) takes a fresh Square (Superglide) or a fresh jump press (Glide, when the dodge
  isn't there for it: `Character::air_dodge_available`), and in the dodge (`StateName::AerialDodge`)
  at once instead of only while falling.
- `kh2_air_dodge` keeps its last answer when the controller is busy (a failed `try_lock` read as
  "no dodge", so neither the borrow's hide nor the dodge hide held for that step).
- Logging for the dodge's first-frame glide request (still open): when KH1's step in
  `shared_control` asks 0x71..0x74, the log has the button words KH1 saw and the real ones, the
  hide, `AIR_DODGED` / `DODGE_JUMP_UP`, and the state flags.

### 2026-10-21: Stage 319 (testing branch: glide only after the Aerial Dodge, on the jump button)
- The user: KH1's glide request on the Aerial Dodge's first frame (t318 traces: KH1's air step asked
  its glide motion, 0x71 / 0x73, the frame KH2 borrowed him for the dodge) isn't wanted: "glide can
  be requested after aerial dodge only (if its equipped)", and "hold off on normal glide on
  square". `shared::control`: with the Aerial Dodge equipped, KH1 sees no jump in the air (past the
  rise) until the dodge is used, nor while the dodge's press is still held (`DODGE_JUMP_UP`): the
  glide, KH1's own held jump, needs a fresh press after the dodge. Without the dodge equipped
  nothing is hidden. Landing resets it (`AIR_DODGED`).
- Glide on Square is off: `settings::glide_on_square` is always false (the `glide = square` line
  is commented out, one line to bring back); the Square glide code (`takeover::shared_frame`,
  `glide_button`, `shared::control`'s hide) is untouched behind that flag. KH2's controller's own
  glide button is the jump again.

### 2026-10-21: Stage 318 (testing branch: the fall after an Aerial Dodge blends from the dodge's pose)
- The user: after an Aerial Dodge "goes back into the jump animation for a frame and soras feet
  seem to glitch out before snapping to his falling pose". The t317 draw columns: the dodge is
  drawn by KH2's controller (`draw_route` kh2_controller), then KH1's fall (slot 10, KH2 motion 4)
  starts with a blend from `Follow::last`, which only our clips set: the High Jump pose from before
  the dodge, 1.5-3 s old (blend 0.00 on the first frame, so the old jump pose showed). Now the
  controller's pose is stored in `Follow::last` each time it is drawn, and a clip blends only from
  a pose under 0.25 s old (as the slot path already did).
- `draw_age_ms` was the age at the time the trace was written; it is now the age when the frame
  was recorded.

### 2026-10-21: Stage 317 (testing branch: the trace records what is drawn on Sora)
- The user: asked for more in the trace to see Sora's model problems (the one-frame jump / glide
  pose at the start of an Aerial Dodge). `pose::wanted` now notes what it chose (`pose::drawn`):
  the new trace columns `draw_route` (kh1_own: KH1's animation, nothing of ours; kh2_controller:
  KH2's controller's pose; follow: a KH2 clip drawn for KH1's slot), `draw_slot` (the KH1 slot
  of that draw), `draw_play` (Motion / Swing / Slot, follow only), `draw_kh2_motion` (the KH2
  motion drawn), `draw_clip_frame` (frames into it), `draw_blend` (the blend into it, -1 none),
  `draw_restart` (the clip started this draw) and `draw_age_ms` (how long ago it was drawn: a
  large age means the row's draw is stale). The log also gets a "draw: ..." line whenever the
  route, slot or motion changes (up to 1500).
- Nothing about how he looks changes.

### 2026-10-21: Stage 316 (testing branch: Sora can act after an Aerial Dodge again)
- The user: "sora now gets locked out of moving after aerial dodge" (traces t315). Stage 315's
  fall request at the hand-back (`shared_give_back`) raised KH1's "action running" bit (bit 4 of
  the actor flags): `0x28007` for the whole fall and landing, where it was `0x28003` before.
  The bit is cleared right after the request, as the code does elsewhere for KH1's idle request.

### 2026-10-21: Stage 315 (testing branch: KH1's fall pose at the end of KH2's move in the air)
- The user: after an Aerial Dodge "sora's feet seem to glitch out before snapping to his falling
  pose". The traces (t314, Aerial Dodge out of KH1's and KH2's High Jump): KH1's motion request is
  0x07 (its landing, slot 11) all through the dodge and for the 1-2 frames after KH2's move ends,
  until KH1's air step asks for its fall, 0x06 (slot 10). The 0x07 is the landing KH1 queues when the
  borrow sets its state to 0 (`set(actor, 0)`). `shared_give_back` (Back::Air) now asks KH1's
  fall request (`0x14029D7F0 (actor, 6, 0)`) at the hand-back when the request is still 0x07.
- Line endings: the testing branch's files are CRLF; stages 313-314 wrote LF into `AGENTS.md`,
  `lib.rs`, `shared.rs`, `pose.rs` and `combat.rs`; put back to CRLF.
- Not done yet (the user: wait, KH1's glide logic first): the Aerial Dodge's first frame still
  shows KH1's last jump slot (12 / 13, now the High Jump clip) and the 0x71 glide slot for a frame.

### 2026-10-21: Stage 314 (testing branch: KH2's High Jump clip on KH1's High Jump)
- The user: "kh2 high jump is not using kh2s high jump animation". The traces (KH1 High Jump,
  height ~235: requests 0x9 / 0xa / 0xb / 0xc, slots 12 / 13 / 10 / 11; KH1 plain jump, height
  ~185: requests 0x4 / 0x5 / 0x6 / 0x7, slots 8 / 9 up) show the old comments had the pairs
  backwards (8 / 9 "high jump", 12 / 13 "jump"). `pose::play_for` now draws KH2's High Jump
  (motion 201) over slots 12 / 13 and the plain jump over 8 / 9; the fall loop takes over from
  either when KH2's arc falls, and a missing High Jump clip falls back to the plain jump.

### 2026-10-21: Stage 313 (testing branch: a KH1 air swing out of KH2's jump hops, it doesn't keep the jump's rise)
- The user's traces (physics KH2, KH1 air combo): a swing started just after KH2's jump took off
  kept the jump's remaining rise (`kh1phys::keep_momentum` takes KH2's arc when it rises more than
  the swing's hop), so Sora climbed from ~50 to ~214 while swinging; at physics KH1, and with
  KH2's own air swing (its hop replaces the jump's rise, ~+12), he doesn't.
  `kh1_air_attack_started` takes `keep_rise`: false for KH1's air attack out of KH2's jump arc on
  KH1's air state (`combat::command`'s first branch): the swing's own hop and arc, the faster
  sideways speed kept. True, as before, out of a KH2 move (an Aerial Dodge).

### 2026-10-21: Stage 312 (testing branch: KH2's air swings in KH2's order as KH1 slots)
- The `select_triangle` trace (t311): the three air swings played from slots 102 / 103 / 104, but
  stage 310 had built them as 183 / 184 / 181, not KH2's own air order. KH2's air combo is
  181 <-> 182 with 183 as the finisher (`docs/movement-model.md`, the stage 307 trace "Attack
  181 / 182 / 183"). `kh2slots::SPECS` and `pose::Play::Swing` now give 102 -> 181, 103 -> 182,
  104 -> 183.
- The "KH2 would take row ..." log line (`combomods::attack`) still shows 181 for both swings: it asks
  a trial copy of KH2's controller with no attack playing, so the anti-repeat step never fires.
  The log only; it doesn't choose the slots' motions.

### 2026-10-21: Stage 311 (testing branch: `kh1_slots`, on by default)
- The user: "can you change it to be kh1_slots? (kh2_slots sounds like kh2 is taking control, not
  kh1) and yes lets turn it on". The ini key `kh2_slots` is now `kh1_slots`, default `on` (a missing
  key is on; `off` gives the old borrow / draw-over path). An ini's old `kh2_slots` line is
  ignored. `Settings::kh1_slots`; the module is still `kh2slots.rs`.

### 2026-10-21: Stage 310 (testing branch: KH2's air swings as KH1 motion slots)
- The user: "can you add kh2s air attacks as slots in kh1?" `kh2slots.rs` (`kh2_slots = on`, or a
  list) now builds KH2's aerial combo as slots 122..124 after the ground ones, standing in for
  KH1's air combo (slots 102 / 103 / 104, requests 0xCC / 0xCD / 0xCE): 183, 184 and the
  finisher 181 (`pose::play_for`'s mapping). Tier A as for the ground combo: KH1's clock, hit /
  combo / trail windows, blends and sounds; KH2's motion drawn at KH1's time; KH2's sounds and
  effects from `kh2slots::tick`.
- With them built, `shared::kh1_combo(air)` is true at Aerial Base Combo KH2 too (`kh2slots::covers`):
  the Attack in the air stays in KH1's air state instead of borrowing him for KH2's air combo,
  so the borrow's hand-backs (stage 308's KH2 High Jump) don't apply to it. Hits, damage and
  lunge are KH1's (its attack rows), as at the ground combo.
- A request that isn't the slot a spec expects is now logged and that motion left out, not a
  failure of the whole table.
- Untested in game. Look for "KH2 slots: KH2 motion 183 / 184 / 181 as KH1 slot" in the log.

### 2026-10-21: Stage 309 (testing branch: stage 308 reverted)
- The user: "I'm still able to kh2 high jump, can you revert our changes" (trace t308: KH2's High
  Jump, motion 201, again out of KH2's air swing). `kh2_jump_open` is back to stage 307's (free
  states and command windows only; no trial step on a copy of KH2's controller).

### 2026-10-21: Stage 308 (testing branch: a jump out of the end of KH2's move is KH1's, again)
- The user: "do you know why I transition into kh2 high jump" (trace t305, Jump KH1, Physics
  KH2). A KH1 jump, an Attack in the air (Aerial Base Combo KH2: KH2 borrows him), KH2's air
  swing (motion 181) touching down on its last frame at f16228; the jump pressed at f16229 went to
  KH2's controller, which jumped him itself: KH2's High Jump (motion 201, his High Jump ability),
  not KH1's jump. No `Land` came between (the swing's end took the jump), so stage 307's landing
  hand-back didn't apply. `JUMP_OUT` (s261: a fresh ground jump where KH2's move can be left is
  KH1's own) asked `kh2_jump_open`, which knew only free states and command windows; the swing's
  end is neither, but KH2's own action takes a jump there (its cancel window, `f >= length`).
  `kh2_jump_open` now also tries the press on a copy of KH2's controller: if it would jump, the
  jump is KH1's. (The Aerial Dodge after it is by design: the jump press in the air is KH2's.)

### 2026-10-21: Stage 307 (testing branch: KH1 lands KH2's air moves in its own states)
- The user: KH1 should handle landing KH2's attacks with its own internal animation state instead of
  giving it back (the `select_triangle` trace: Attack 181 / 182 / 183 in the air, then KH2's `Land`
  for 26 frames with KH1 sitting in state 0 / motion 0 the whole time, then idle gave him back).
  `shared::give_back` now returns `Back::Land` the moment KH2's controller enters `Land` on the
  floor: `shared_give_back` sets KH1's fall state (2) with the mirrored momentum, as for `Back::Air`,
  and KH1's own state step lands him (its landing request and slot 11, its hold, its can-act
  rules), at either Jump style (KH2 draws its land over slot 11 at KH2). A command or press of KH2's
  still waiting keeps KH2's `Land` as before. KH1's own air casts landing into KH2's hold
  (stage 163, `land_now`) are unchanged.

### 2026-10-21: Stage 306 (testing branch: KH2's landing after an air move plays out)
- The user's trace (`select_triangle`): an air attack touched down into KH2's Land (motion 5), but
  `shared::give_back` counted `Land` as over (grounded, no combo) and handed him to KH1 two frames
  in, so the landing hold never showed. `Land` is out of the give-back; it ends into idle (or walk
  / run after the hold), and those give him back.

### 2026-10-20: Stage 295 (testing branch: KH2's physics in KH1's fields; KH2 animations as KH1 motion slots)
- The user: "kh2s physics gets written into kh1s internal physics system instead" (a KH1 air move
  out of a KH2 air move had no momentum). KH2's every 60 Hz step is mirrored into KH1's own
  velocity (`+0x3C0`) and arc (`+0x3B0..+0x3BC`) (`shared::physics::mirror`, `kh1phys.rs`); KH1
  taking him in the air starts from them; KH1's air attack out of KH2's motion keeps the faster
  sideways and the higher rise; KH2 taking him from KH1's air starts at KH1's speed. One switch
  point for the later setting (`shared::physics::source`, `Rule`). `docs/kh1-physics.md`.
- The user: "our kh2 animations can run entirely through kh1s animation system as new animation
  slots". KH1 resolves request -> slot (`+0x1D8`, 321) -> motion (`+0x1D4`, 118 handles); Sora gets
  longer copies (`docs/kh1-anim-slots.md`). KH2's ground combo (151 / 152 / 151 / 154) as KH1 slots
  118+ with KH2's windows as KH1 motion events, KH1's requests redirected at Base Combo KH2
  (`kh2_slots = on | <motions>`, default off; `kh2slots.rs`, `kh1_data::slot`). The pose is still
  drawn by us at KH1's clock.
- KH1's glide is slots 75..78 (requests 0x71..0x74 / 0x76..0x79), not 113..116 (attacks): the
  KH2 glide never showed at Glide KH2. Fixed.
# AGENTS.md

Notes for anyone (human or AI agent) working on this repo.

## Ground rules

- Never commit game files, extracted tables, dumps, Ghidra databases or decompiled code. The
  `.gitignore` is a whitelist: add a pattern only for source, docs or manifests.
- Put scratch output (table dumps, binaries) in `local/`. It is ignored.
- `combat-core` must stay free of KH2 knowledge and engine dependencies. KH2-specific
  mapping belongs in `kh2-data`.
- Write down every rule's source and confidence in `docs/combat-model.md`.
- Every change to the source gets a new stage: an entry in the development log below and
  `VERSION` in `crates/kh1-hook/src/lib.rs` bumped to match (the user: "every time a change is made
  to the source code make sure the version updates"). On the testing branch the version is
  `t<stage>` (`t300` on), not `s<stage>`; `tests/version.rs` checks it.
- When code is ported from another project, credit it in `NOTICE` and `CREDITS.md`
  and keep its licence.

## Several sessions at once

Up to one lead and three helper chats can work together. Follow `docs/WORKFLOW.md`: lanes
by crate, claims on the board in `local/board/`, delivery to the PC's `main` under a lock.

## Reverse-engineering notes

- Ghidra runs headless on a private copy of the user's executable, outside the repo. Commit
  only plain-language findings (in `docs/`) and our own Rust. Never commit decompiler output,
  Ghidra projects or byte dumps.

## Testing against the game

```
KH2_GAME="D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe" cargo test
```

## Development log

### 2026-10-21: Stage 305 (testing branch: KH2's jump hands over to its fall loop at the apex)
- The user: "letting go of jump still doesn't transition kh2 soras jump into his falling
  animation" (the "2nd animation" of stage 304 was the fall). Trace t304: released at f5680, the
  hop's apex 5 frames later, but KH1 kept its apex slot 9 (drawn as KH2's jump) to f5699 before its
  fall slot 10. KH2's own rule (`kh2_rig`): the jump plays while rising, then the fall loop. `pose`
  now shows KH2's fall (motion 4) on KH1's jump slots (8/9, 12/13) once KH2's arc on KH1's air
  state is past its apex (`shared::physics::kh2_arc_falling`, set in `air_step`), for every jump,
  a full one too. Stage 304's skip of the jump clip is taken out.

### 2026-10-21: Stage 304 (testing branch: a released jump shows KH2's jump clip's apex part)
- The user: "kh2 jump doesn't transition into its 2nd animation when you let go of the jump button,
  I only saw the 2nd animation when I did a max jump at the end". The trace (t303): the early
  release now cuts the rise to KH2's hop, but `pose` draws KH2's jump clip (motion 3, KH1's jump
  slots 8/9 and 12/13, one group) at its own clock from the takeoff: the hop's ~31 frames ended
  before the part a full jump shows round its apex (from about frame 28-33), then the fall loop.
  On the cut (`shared::physics::air_step`), `pose::jump_released` moves the clip on so the full
  jump's apex frame (3 frames of KH1's takeoff + its rise) comes with the hop's apex. KH2's own
  controller's jumps (`kh2_rig`, KH2 has him) are unchanged.

### 2026-10-21: Stage 303 (testing branch: KH2's early-release hop on KH1's jump, again)
- The user's trace (t302, physics KH2, Jump KH2): a standing High Jump on KH1's states with KH2's
  arc went to its full 235 though the jump button was let go 9 frames into the rise; KH2 cuts the
  rise to a hop there (`character_core::release_hop`, after 6 frames). The log showed `air_step`
  taking KH1's own rewrites of its arc fields during the rise (every other frame) as KH1 starting
  one of its own air moves: the arc was rebuilt from KH1's each time (`kh1_arc_on_kh2`), its time
  starting over and `holdable` lost, so the release check never passed. A plain jump's rise of ours
  (holdable, KH1's action flag 4 off) now keeps KH2's arc through those rewrites.
- Not a bug, from the same trace: the probe's "drawn at" y one frame behind `+0x490` is the
  frame-start copy (`takeover` writes it), the model is where the actor is.

### 2026-10-21: Stage 302 (testing branch: the trace clip's length is set in the ini)
- The user: "can you change it so that the csv clip can be adjusted in the ini file". New ini
  line `trace_seconds = 10` (1 to 120, 10 when missing; `settings.rs`): how many seconds of
  Sora's frames the trace keeps and writes on Select + Triangle or an anomaly (`trace.rs`; the
  row cap follows it at 120 rows a second). The anomaly cooldown stays 10 s.

### 2026-10-20: Stage 301 (testing branch: KH1's air attacks out of KH2's air moves get their lunge)
- The user's traces (t299): KH1's aerial combo swings start at a fixed sideways speed along his
  facing, 7.80 / 3.90 / 11.70 a frame for motions 0xCC / 0xCD / 0xCE, slowed x0.975 a frame (the
  finisher also rises 40); its finisher goes ~280 forward. Started out of KH2's air move (KH2's
  combo with the combo style KH1's), KH1's start left `+0x3C0` at 0: no sideways travel at all, he
  rose and fell where he was, under either Physics setting. The user: "lets give sora a fixed 11.7
  starting speed to match how the move would normally behave in a kh1 combo, this bug also appears
  any time a kh2 air attack transitions into kh1 attack". `shared::physics::seed_kh1_lunge`: after
  KH1's physics frame, within 4 frames of such a start (`combat::command`), the first frame of a
  KH1 aerial swing with no sideways speed gets that swing's lunge along his facing, decayed to the
  swing's frame (`+0x3B0`). It starts a frame late (its first frame's ~12 units are not made up).
- The trace's `kh2_*` columns are empty on frames KH2 doesn't have him (they showed KH2's last
  move, stale, all through KH1's).

### 2026-10-20: Stage 300 (testing branch: versions are `t<stage>`, bumped with every change)
- The user: "every time a change is made to the source code make sure the version updates, also
  instead of S for our lettering lets do T for test branch so now we should be at T300". `VERSION`
  is `t300`; the version test expects `t<latest stage>`; the rule is in Ground rules above.
- Stage 299's trace: the row built in `takeover::trace_frame` was missing the new `config` field
  (the build failed); it is set there (`trace::frame` fills it).

### 2026-10-20: Stage 299 (testing branch: Sora's per-frame trace)
- The user: "have the log keep track of kh1s soras position, kh2s soras position, the current
  anim state, etc... anything that would help you debug soras current actions and physics without
  me having to show you a video". `trace.rs`: a row for each of Sora's physics frames (who has him,
  where he is, how far he moved and how far KH2 asked, KH1's actor: state, motion, height above
  the floor, collision flags, velocity, arc; KH2's controller: state, motion, frame, air, on top,
  facing, combo, lock-on; Drive Form; the pad; the log lines of that frame). The last 10 s are kept
  in memory and written to `kh1-kh2-traces\` as a CSV on Select + Triangle (the user's pick; KH1
  doesn't see Select while Triangle is held, its first-person view) or, a second later, on their
  own when something looks wrong (a jump of more than 150 a 60 Hz step, 30 under the floor for 3
  frames, KH1 and KH2 trading him 8 times in 60 frames, a NaN). Ini `trace = on | always | off`
  (`always`: every row to `kh1-kh2.trace.csv` too).
- The user: "another important thing to add to the csv file is what settings are set in the config
  menu". Each row has the Config menu's KH1 / KH2 rows on that frame (Camera Style, Physics, every
  move style) and `physics_now`, whose numbers moved him (KH2's in a Drive Form).
- Every `kh1-kh2.log` line starts with the game frame it was written on (`[f123]`, the trace's
  `frame`), so the two line up.

### 2026-10-20: Stage 298 (testing branch: the Physics KH1 / KH2 setting)
- The user: "In the config there's not a setting to switch between KH1 and KH2 physics". Ini
  `physics = kh1 | kh2` (default kh2) and a Config row after Camera Style, live. Styles now pick
  only animations; `shared::physics::source()` picks the numbers (KH2 in a Drive Form). `kh2`: KH2's
  numbers on every base Sora move, KH1's own moves included; `kh1`: KH1's own states unmodded and
  KH2's controller given KH1's gravity / fall cap / jump / run / air steering (new
  `character-core` `air_accel`). The old hair switch `physics = on/off` is now `hair_physics`
  (an old ini's value carried over). KH2's air drift on KH1's states was applied twice as often
  as KH2 does; fixed. Choices for one-sided moves: `docs/kh1-physics.md` 7.
### 2026-10-07: milestone 1
- Surveyed existing work: OpenKH (format docs and C# readers), GovanifY/kh2 (PS2 decomp, early
  stage), KHPCPatchManager. Chose documented formats; no decompiling was needed.
- Ported the HED/PKG reader. The cipher's substitution table is located in the user's exe at
  runtime by a sliding-window permutation search, then checked against a SHA-256.
- Parsed 00battle.bin. The counts on the user's install match OpenKH's docs (atkp 2713, enmp
  229, ...). Live and backup copies are identical.
- Derived from the data:
  - ATKP power is a percentage (most common value 100; Thunder spells use 375 = ×3.75)
  - the LVPM level-99 Defense of 51 matches the community example
  - ENMP min/max damage × HP is nearly constant, which suggests a fraction of HP
- Open: the clamp reading, critical_adjust scope, element ids 4–6. Next: an MSET/ANB motion
  trigger reader to get hitbox timing and Sora's combo attack ids.

### 2026-10-07: milestone 2 (movement + combat playground)
- Added readers for 03system.bin `pref` (prty/fmab/plyr/sstm), PTYA combo rows, MSET/ANB motion
  headers, root-motion F-curves, motion triggers and model collision.
- Findings on the user's install:
  - trigger frames are 60 Hz and relative to the clip start
  - Sora's keyblade hitbox windows live in `W_EX010.mset` at the same slots as `P_EX100.mset`
  - root motion: Dodge Roll 438 units, A300 lunge 92
  - fmab records 0–3 = growth LV1–MAX
- Built `character-core` (engine-agnostic controller) and `kh2-bevy` (capsule playground).
  The playground was verified under Xvfb with `--synthetic --autoplay --screenshot N`. The
  real-data path was verified on the user's machine with `kh2-combat sora`.
- Open: gravity (estimated), the combo `sub` chain, skeleton-accurate hitboxes, enemy attacks.

### 2026-10-08: Phase 1 from the executable
- Ghidra 12.1 headless on the user's exe. MSVC RTTI names the action classes, so the
  action vtables (jump, glide, air slide, hop, idle) were easy to find.
- Implemented from plain-language notes:
  - the 30 Hz velocity tick
  - the double-sine jump with gravity 20/49
  - air control 0.9/0.95
  - glide, Quick Run and Aerial Dodge formulas
  - the PTYA selector (table order, `sub` anti-repeat, kind/flag semantics, `combo_offset`)
  - the hit-driven combo counter
  - jump-attack arcs and the lunge factor
  - the damage routine (stat source byte, base-term clamps, ceil rounding, difficulty table,
    boosts, Cure)
  - enemy status init
  - ATKP level variants
- Removed: estimated gravity, the approach-assist hack, the `--clamp` and `--power-scale`
  switches, and the score-based selection.
- Verified on the install:
  - the jump peaks at 185 on frame 33
  - whiffing alternates 151/153
  - connecting gives 151→153→155
- Open: jump hold, stick-tilt mapping, landing, cancel points, auto-target, timer units.

### 2026-10-08: skeleton, animation and a mannequin
- Aerial Dodge is the jump button in mid-air (KH2), then Glide; it was on the action button.
- New readers:
  - MDLX skeletons
  - full interpolated motions (initial pose, F-curves with pre/post behaviour, IK helpers,
    joint table, constraints, activations, limiters, expressions, external effectors)
  - `00objentry.bin`
  - `03system.bin` → `sklt` (weapon bones)
- New crate `kh2-anim`, implemented from plain-language notes on the exe's motion evaluator:
  - key times are 60 Hz
  - the unscaled/scaled matrix pair
  - constraints and limiters
  - one- and two-bone analytic IK
  - the expression language
- Root motion is now sampled at 60 Hz key time. Lunges were half length before: A300 now
  travels 150.
- Playground: Sora is a capsule mannequin on his real bones, playing his real motions, with
  the Keyblade attached to bone 178. Hits use the real bone-attached attack spheres.
  `--data-dir local/re` runs from extracted files (research only).
- Verified locally:
  - every Sora motion poses without NaNs
  - idle stands at y = 0
  - screenshots of idle, run, attack, jump, roll and glide look right
- Open: per-move blend times, foot ground adaptation, head look-at, the Shadow's own
  skeleton.

### 2026-10-08: playtest fixes
- Stick left/right was mirrored (screen-right is −X when looking along +Z).
- Read from the exe:
  - **Variable jump height:** an early release swaps the rest of the rise for a 5.1-unit
    hop, after 6 frames and before apex − 5.
  - **Quick Run** lasts only while the button is held, and is ground-only.
  - **Dodge Roll's** frame-10 event chains into Quick Run while the button is held.
- Air combos: whiffs now advance the count, per the user's knowledge of the game (the exe's
  rule for this isn't found yet; ground stays hit-only).
- Rig:
  - **End-of-action zoom fixed:** the last frame was posed as a free loop without the root
    offset, and blending then smeared that jump.
  - Bones are now kept in object space.
  - The Keyblade's blade/keychain orientation is fixed.
  - New simplified mannequin.

### 2026-10-09: Phase 2 (skeleton-accurate hitboxes)
- **Hit detection, from the exe:**
  - Attack windows collect the attack- and guard-type shapes of their collision group.
    Defenders use their hit-type shapes.
  - The overlap test: sphere, column, ellipsoid; cubes never hit.
  - The 1.5 × radius sweep between frames.
  - Implemented engine-agnostically as `combat_core::{Shape, AttackVolume, first_overlap}`,
    with unit tests.
- **Data:**
  - `kh2_data::sora::BoneShape` keeps every MDLX collision entry: type, kind, bone or root,
    offset, lift, radius, height.
  - Guard windows keep their group (`WindowKind::Guard { attack, group, weapon }`).
    `Output::guard_boxes` lists them.
- **Blending, from the exe:**
  - blend lengths of 8 (default and locomotion), 16 (glide and the fall helper), PTYA
    `blend_time` for attacks (0) and 0 for queued follow-ups
  - the clock starts at 2
  - walk↔run phase sync
- **Playground:**
  - hits use the game's test, with the sweep
  - F1 attack/guard, F2 hit, F3 skeleton lines, F4 other collision types
  - `--show-hurt`, `--show-skeleton` and `--show-other` for screenshots
- **Open:**
  - clash table
  - wall-hit ray
  - the motion freeze during a blend
  - group-enable bits from the collision header
  - enemy collision (Phase 3)

### 2026-10-09: playtest fixes, ledge grab, wall hits, Phase 3
- **Landing (exe):** landing hands over to the idle action. The stick takes over at once, and
  with a neutral stick Sora stops dead instead of sliding.
- **Arena:** a box top crossed during a fast fall now counts as ground, and the support and
  push-out radii match. This fixes slipping off box tops.
- **Ledge grab (exe):** the search box, timing and placement, using `hang_*` from prty/sstm
  and the type-4 radius. Motion 7 hangs; motion 8 climbs.
- **Wall hits (exe):** the BG-hit ATKP flag, the ray from the previous centre, the
  floor-check ray, and the recoil (motion 41/43, 50 push or hop). The swing counts as
  connected.
- **Reactions and clashes (exe):** the shared reaction routine (groups, motions, hit-back
  distance, launch physics), the 7×7 clash table with angle checks, and the party and enemy
  clash reactions.
- **Enemies:**
  - `kh2_data::enemy` loads the Shadow (M_EX010) from the install
  - `character_core::npc` runs it
  - the playground's Shadows attack on our timer, hit Sora and get guarded
- **Open:** enemy scripts, weight, revenge, Sora's damage-lock length.

### 2026-10-09: Phase 4 (HUD, menus, abilities, magic)
- **Playground changes:**
  - one Shadow by default (`--enemies N`)
  - the ledge hang waits for a fresh push or jump before climbing (our change)
  - Aerial Recovery (jump during a launch)
- **Enemy weight (exe):** the body push is split by objentry weight.
- **03system item/cmd tables and msg text:**
  - the command menu (Attack, Magic, Items, Drive)
  - the ability menu with real names and AP and the level's AP cap
  - HP/MP gauges
- **Correction (exe):** the ATKP row "level" byte is the defender's consecutive-hit count
  from the same attack group, not the attacker's level. It is now implemented with the
  game's per-object hit record (`combat_core::HitRecord`: hit-once bits, group/count, a
  30-frame group timer for the player, 4 re-hit interval slots). Sora's and the Shadows'
  swings use it too.
- **New crate `kh2-bdx`:** the BDX script machine, written from the interpreter. The trap
  names come from OpenKH's list, whose 828 argument counts match the executable.
- **New crate `kh2-magic`:**
  - spell casting (trigger 20 with caster, resources, sub and the `magi` record; trigger 21
    at the cast motion's magic-cast event)
  - the 59 traps the six spells use
  - spell attacks (the game's magic attack shape and sweep, duplicates sharing ids)
  - effects (PAX elements by id, bone codes, lifetimes from the DPD emitter headers)
  - a y-down/y-up flip at the boundary
- **Reaction 5 (exe):** Magnet's orbit pull.
- **Verified:**
  - every spell file at every level runs to the end headlessly (`kh2-magic` test with
    extracted files)
  - in the playground: Firaga hits 5 times (alternating 8/10-frame re-hits, a 175% final);
    Blizzaga's shard; Thundaga's bolt; Curaga heals; Magnega lifts the Shadow into an orbit;
    Reflega stops a swipe and counters with 574 ×5 and a 667 burst
- **Open:**
  - the MP Charge rate
  - the target search without lock-on
  - Drive Form variants
  - enemy scripts (the VM is ready)

### 2026-10-09: Phase 4 finished (map collision, camera, Drive Forms, reaction commands)
- **New crate `map-collision` (exe):** the game's background solver.
  - one sphere (r 90, centre 115) plus a lower sphere for walls
  - sub-stepped push-out: faces, then edges
  - anti-tunnelling segment test
  - the floor ray with a 24-unit step-down
  - the 49% ceiling rule with a 16-frame air-command lock
  - Step-up 25 and the 38.5° slope limit come from the sphere's geometry.
  - The arena is now polygons: stairs, ramps, a floating slab.
- **New crate `kh2-camera` (exe):**
  - the field camera: distance/height curve, swing-behind, spring smoothing, wall pull-in
  - the lock-on framing
  - target switching
- **Drive Forms (exe + data):**
  - each form's object from objentry/pref/PTYA/wmst, with two weapons where the data has them
  - `character_core::drive`: the gauges, costs, Anti roll, levels and growth
  - Sora's Drive motion and invulnerability, the arrival motion, revert, magic subs
  - Wisdom's shots from its model's BDX script (`Magic::object_effect`, trap
    `obj_effect_start`)
- **Reaction commands (exe + data):**
  - RC areas from type-5 collision with group switching (motion trigger 4)
  - priority/score selection, the low-HP Drive offer
  - capture motions read from nested motion sets
  - the Soldier's Cyclone, as its script runs it
- **Correction:** the "Shadow" was a Soldier (M_EX010). The real Shadow is M_EX020.
- **Open:**
  - enemy scripts
  - other enemies' RCs
  - items, form keyblade stats
  - the MP Charge rate

### 2026-10-12: KH1 port started (KH2 melee in Kingdom Hearts Final Mix PC)
- The user's request: KH1 stays KH1 except the melee combat, which becomes the KH2 rewrite's;
  KH2's abilities and combo modifiers go in KH1's Ability menu. The user's choices: KH1's own
  melee abilities are swapped for their closest KH2 counterparts in the same menu slot and
  level-up spot (the extra KH2 modifiers are added as new abilities); KH2 modifiers are learnt
  through KH1's level-up tables (Warrior / Guardian / Mystic) and equipped with AP.
- New crate `kh1-data`: KH1's packages (`kh1_first..fifth`, same format and cipher as KH2's,
  key table from the KH1 executable; `BackupImage/` originals first so a randomizer's edits
  don't leak in). CLI `kh1-extract --kh1 <install> --file a,b --out <dir>`. All 601 names in
  `kh1_first` resolve from OpenKH's `kh1idx.txt` / `kh1pc.txt` lists (the user's install).
- Sora is `xa_ex_0010.mdls` (8 sections; the first is `MOBJ`) and `xa_ex_0010.mset` (3
  sections; the first is `MMTN`, a motion table); battle table `btltbl.bin` (ability level
  tables at `+0x3BF8` and two more, per the Archipelago scripts).
- The KH1 executable is x64 and imports `DINPUT8.dll` (a proxy DLL; the
  folder is shared with KH2, BBS and Re:CoM, so the DLL must only act in KH1). The user runs
  OpenKH's Panacea (`DBGHELP.dll`) and LuaBackend with an Archipelago randomizer: stay
  compatible.
- Scope widened (the user, same day): KH2's movement abilities too, and KH1 Sora uses KH2's
  animations for everything except summons, Drive Forms (none in KH1) and magic. So KH2's
  controller drives Sora outright (idle, run, jump, glide, dodge, guard, hurt, attacks), as
  in KH2's combat mode, with KH1's collision underneath; KH1 keeps magic, summons, items,
  cutscenes and menus. Proposed (to confirm in play): KH1's story abilities map onto KH2
  growth (High Jump x1-3 -> High Jump LV1-3, Glide -> Glide LV1-2, Superglide -> Glide
  LV3/MAX, Dodge Roll -> Dodge Roll LV1); Quick Run, Aerial Dodge and the higher growth
  levels go into the level tables; Mermaid Kick and KH1's special attacks (Sonic Blade, Ars
  Arcanum, Ragnarok, Strike Raid) keep KH1's animations, having no KH2 counterpart.
- KH1 Sora's skeleton (`kh1_data::mdls`, read from the user's `xa_ex_0010.mdls`): 293 bones,
  0x30 bytes each (KH2's MDLX bone packed tighter: scale, Euler rotation, translation, links
  parent / sibling / child in 10-bit fields), bind pose a T-pose like KH2 Sora's (256 bones;
  KH1 Sora's hips at 78.9 against KH2's 88.5). `examples/skeletons.rs` prints both bind poses
  side by side for the retarget map.
- New crate `kh1-hook` (64-bit `dinput8.dll`, `-Zbuild-std` for `x86_64-pc-windows-gnu`):
  stage 1 only forwards DirectInput, and in KH1 alone logs a heartbeat to `kh1-kh2.log`
  (which other mods are loaded). Installed in the KH 1.5+2.5 folder.
- KH1's skeleton pass (Ghidra, the user's Epic exe; ASLR, so base + RVA): `0x140391E20`
  `(world, instance, time f32, motion)` samples the motion into per-bone arrays and builds
  the bone matrices (`0x140390260`); `0x140392710` `(world, instance, flags)` is the variant
  with ground IK (called by `0x14029E490` with the actor's instance at `actor+0x15C`). The
  instance: `+0` the model header (bone count, bone-table handle); handles to 0x40-byte
  per-bone matrix arrays at `+0x1B0..+0x1C0`, translation / rotation arrays at
  `+0x1C4` / `+0x1C8`. Game handles are 32-bit: `[exe+0x2EE4730][(h & 0x7FFFFFFF) >> 25] |
  (h & 0x1FFFFFF)` (`0x14038B230`). Matrices are D3D row-vector layout (translation at
  +0x30), i.e. glam's column-major memory.
- `kh1-hook` stage 2 (probe): inline hooks on both (x64: `jmp rel32` to a stub near the exe,
  trampoline with the moved prologue); logs Sora's (293 bones) arrays; Insert cycles a
  T-pose override through the five arrays to learn which one the game draws.

- Loading (the user: KH1 crashed launched directly; then the Epic launcher would not start
  with our `dinput8.dll` in the shared folder): launched without Epic's arguments KH1 crashes
  on its own (`-epiclocale` read as null in `0x1400D9BD0`). The proxy is out of the game
  folder. The DLL is now loaded by Panacea (OpenKH's `DBGHELP.dll`, read in Ghidra): in KH1
  only, it `LoadLibrary`s every `*.dll` in `<mod_path>\kh1\dll\` and calls `OnInit(wchar_t*
  mod folder)` / `OnFrame()` if exported (`kh1-hook` exports `OnInit`, which logs the route).
  Installed as an OpenKH mod (`crates/kh1-hook/openkh/mod.yml`: `dll/kh1-kh2.dll`, method
  copy) in the user's `mods/kh1/kh2-combat`, enabled in `mods-KH1.txt`, plus a copy in the
  built `mod/kh1/dll/` so it works before the next Mods Manager build.
- Probe fix (the user: Insert did nothing; log: loaded by Panacea, hooks in, but Sora never
  seen): the instance's `+0` points at a slot that points at the model header, whose bone
  count is a u16 (`0x140390260`: `**(short **)*instance`); we read one level short. Every
  model size posed is now logged once. `+0x1B0` holds the sampled channels (scale `+0`,
  rotation `+0x10`, translation `+0x20` per 0x40-byte bone), not matrices; `+0x1B8` the built
  matrices; the T-pose test only writes arrays whose bones look affine.
- Deploying `kh1-hook`: while KH1 runs, Panacea's copy in `mod/kh1/dll/` is locked and a
  write to it does not land (the upload reported success; the second test ran the old
  build). Rename the loaded file aside (`*.dll.loaded-old`, not loaded: Panacea takes
  `*.dll` only), copy the new one in, and check its SHA-256 on the PC after every deploy.
  The user's two crash dumps of 2026-10-13 (KH1 and the launcher, the launcher without our
  DLL) are both the null `memcpy` of a start without Epic's arguments.
- Stage 2 result (the user's Insert test, log): Sora is the 293-bone model posed by the plain
  pass (also by the ground-IK pass). Writing into `+0x1B4` gave a clean T-pose: that is the
  world matrix array the game draws (y-down world). `+0x1B0` holds the sampled channels (scale
  `+0`, accumulated scale `+0x10`, Euler `+0x20`, translation `+0x30` per 0x40-byte bone), kept
  between frames (writing it made him huge); `+0x1B8` is each bone's matrix relative to its
  parent (bone 3's matches its channels with KH2's Euler order). The T-pose lay sideways
  because it was placed from the bind's root rotation; in motion KH1's root is the hips with a
  yaw. Model → world is now `+0x1B4[0] * +0x1B8[0]⁻¹`, read each frame.
- New `kh1_data::retarget` (ours, from the two bind poses): KH2 Sora's pose on KH1 Sora's
  skeleton. Both bind in a T-pose facing +z with the same chain layout; 113 KH1 bones take
  their KH2 counterpart's model-space turn from rest (legs, spine, neck, head, arms, fingers),
  the rest turn with their parent; KH1's bone lengths, hips scaled by the hip-height ratio.
  `examples/retarget_check.rs` (KH2=1 for KH2's own pose) checked idle and run against KH2's
  own poses in side / front plots.
- `kh1-hook` stage 3: KH2 Sora loads on a thread from the KH2 install in the same folder
  (backup originals if the Mods Manager made them); Insert cycles off / KH1 T-pose upright /
  KH2 idle / KH2 run, written into `+0x1B4` after the game's pass (the Keyblade still follows
  KH1's hand). Logs a check that `+0x1B8` chains to `+0x1B4`.
- Waist fix (the user's screenshot of the in-game idle: the waist contorted). KH2's root,
  pelvis, spine, first hip bone, ankles and arm bones carry another built-in orientation in
  its motions than in its bind pose (the idle's pelvis 210° from rest, ankles ~115° off their
  foot every frame), so copying their turns from rest spun KH1's belt, pouches and shorts.
  `kh1_data::retarget` now measures every turn from KH2's joint positions: torso frames from
  bones riding on each part, limb segments aimed joint to joint and rolled by the knee /
  elbow direction (foot / knuckle line while nearly straight), fingers by segment.
  `examples/roll_probe.rs` checks it (leg rolls within ~30° of the foot in the idle; the run's
  kicked-back thigh 69° from the pelvis where the foot-rolled version gave 166°).
- KH1 motions (`kh1_data::mset`, from the user's `xa_ex_0010.mset` and the skeleton pass
  `0x140391E20` / channel evaluator `0x14038F9D0`): the `MMTN` table (118 slots for Sora),
  per motion frames / fps / bones / channels / keys, keys `time<<8 | interpolation` (1/256
  frames; hold, linear, Hermite with per-frame tangents). KH1 animates bodies through ~11 IK
  helpers per motion (ground root, body centre, hips with their yaw, pelvis, chest, hands,
  feet; records of scale / parent / rotation / translation) and per-bone flags pick the IK
  chains and their target helpers, so plain channel playback leaves the legs straight. Slots
  read from the helper paths: 0-1 standing, 2-4 walking (?), 5-7 running (looping), 8-10
  jumping / in the air, 11 landing; to be confirmed in play. A motion's counts (frames,
  channels, helper channels, helpers) name its slot (116 signatures for 118 slots; the shared
  pairs 77/78 and 86/87 are not movement).
- `kh1-hook` stage 4: the motion playing is at `instance + 8`; standing / walking / running /
  in-air / landing slots get KH2's idle / walk / run / jump or fall / land (8-frame blends),
  every other slot keeps KH1's animation. Insert cycles follow / off / T-pose / idle / run.
  Slot changes are logged with speed and climb.
- Stage 5 (the user: a split-second vanish going idle → walk/run; requests: jump, glide and
  attack animations, Keyblade and no-Keyblade versions in their situations). Play log
  (2026-10-13): slots 0 / 2 / 5 field idle / walk / run, 1 / 3 / 6 the battle ones (the field
  ones follow attacks back to the field); jump 12 → 13 → 10 → 11; glide 102 / 103 / 104;
  ground combo 98 → 99 → 98 → 101. `pose::play_for` maps them: field slots take KH2's field
  variants (no Keyblade; KH2 has them for idle / walk / run), jump / fall / land, glide / glide
  end, and swings 151 / 152 alternating with finisher 154 (KH2's no-target ground rows),
  each stretched to the KH1 swing's length and with KH2's lunge taken out (KH1 moves him).
  The vanish: some passes leave `+0x1B4[0]` as we wrote it, and model → world read from it
  compounded our own pose (the log's speeds of 10^4-10^5 units/s too); the last placement
  is kept when the root reads back as ours. The Keyblade: a small model (≤ 40 bones) whose
  world matrix holds a fixed relation to KH1's hand over 20 frames while the hand moves is
  taken as held and re-placed from the hand drawn (logged; lets go if the relation breaks).
  `examples/kh1_motions.rs` / `kh2_rows.rs` list both games' motions (scratch research).
- Stage 6 (the user: the vanish still there; animations jumbled; air combos glide; sliding
  in every animation). Log: 102 / 103 / 104 are the air combo (two swings and a finisher),
  not the glide (the user has no glide yet); now KH2's 183 / 184 alternating and 181. The
  vanish: speeds of 10^4 units/s still at every slot change, so KH1's blends between motions
  mix our drawn pose back into `+0x1B4[0]`; placement now comes from the pass's own world
  matrix (y and z columns negated, axes normalised, as `0x140391E20` does with `× -1` at
  `0x140430938`) once it agrees with the bone-derived one for 30 frames (logged either way),
  and meanwhile a jump of over 30 units in one frame is ignored unless it persists 3 frames.
  Sliding: walk / run play at his measured speed over the loop's ground speed (planted heel
  against bone 0: KH2 run 484 units/s, walk 111, field run 427, field walk 134; KH2's own run
  speed 8 / frame = 480 / s agrees), scaled by the hip ratio. Swings still slide with KH1's
  own lunge until KH2's controller moves him. Combo alternation no longer resets on the idle
  KH1 plays between swings (1.2 s gap).
- Direction change (the user, 2026-10-13: "I want the EXACT KH2 combat, not just
  animations"; sliding stayed because KH1 still moved him). KH2's controller
  (`character-core`) will drive Sora: input, movement, physics,
  jump / glide / dodge / guard, the combo system and hit timing; KH1 keeps its world,
  collision, enemies, HP / MP, magic, items, summons, menus. The animation-follow mode
  (`pose::follow`) stays as the fallback until then. Stage 6 log: placement from the pass's
  matrix took; glide is slots 75 / 76 / 77; 117 unknown.
- Ghidra findings for the takeover:
  - Input: every controller type ends in `0x140112C70` `(device state, PADDATA out, config)`,
    KH1's PS2-style pad record: `+0` buttons u16 in PS2 order (bit 0 Select ... 4-7 d-pad
    Up / Right / Down / Left, 8-11 L2 R2 L1 R1, 12-15 Triangle Circle Cross Square; it clears
    opposite d-pad pairs), `+2..+5` stick bytes (order to confirm), `+0x1F` status. XInput is
    imported by ordinal (2 GetState at `0x140113B80`), DualShock / DualSense through HID
    (`0x1401132A0`, `0x140111490`).
  - Floor query `0x1401910E0` `(query)`: `{x, y, z, 1, 0, range, 0, 1}` in, nearest level
    surface vertically within range at `+0x20` (point), `+0x30` face (short, -1 none)
    (`0x140190B00` walks the cells' polygons); used by the ground-IK pass.
  - Actor animation update `0x14029CF10` `(actor, dt)`: `+0x10` position (vec4, to confirm),
    `+0xD0` world matrix, `+0x15C` animation instance handle, `+0x168` motion slot u16,
    `+0x16C` motion time, `+0x1D4` motion table handle.
- `kh1-hook` stage 7 (probe, `probe.rs`): finds Sora's actor through his instance, searches it
  for his drawn position, puts hardware write watchpoints (debug registers on every other
  thread, a vectored handler) on that field for 25 s and logs the writing code by address;
  logs the pad record on change.
- Stage 7 probe result (the user's run): Sora's actor at `0x142D37C80` (party array, Sora
  first; actors are 0x4B0-byte records in a pool from `0x142D37CA0`); position `+0x10`
  (copies at `+0x50`, `+0x100` in the world matrix at `+0xD0`, `+0x260`, `+0x2E0`, `+0x320`);
  the only regular writer is `0x140386D00` (vector add) from the collision mover
  `0x140299350`, ~57 times a second. Pad record: Cross 0x4000, Circle 0x2000, Square 0x8000,
  d-pad as PS2; stick bytes rx, ry, lx, ly (left: 0 = left / up, 0xFF = right / down).
- Ghidra, the actor's frame (`0x140295E50`): think (`0x1402957F0` picks it by object kind;
  `0x1402A1170` for the player and party: the move vector, the state's handler from the
  table `0x142DB4240` (0x40 bytes a state: flags, enter, exit, update, displacement,
  transition, ...; registered by `0x1402AF9F0` from `0x1402AF8B0`), the turn toward `+0x150`),
  physics `0x140298230` (state displacement, platform carry `+0x120`, `0x140297710` move with
  collision, floor `+0x320` / height above it `+0x3D4`, transition), matrix, animation.
  `0x14298D3A0` is set while an event runs. The state at `+0x70` stays 0 through KH1's
  attacks, so KH1's own actions are told by the motion slot instead.
- `kh1-hook` stage 8 (`takeover.rs`): KH2's controller drives Sora's movement. Hooked
  physics step; for Sora, with no event and KH1 showing standing / walking / running (slots
  0-7) or KH2 having him in the air: input (direction from KH1's camera-relative move vector
  `+0x20`, tilt from the left stick, Circle jump, Square KH2's action button), KH2's step
  with KH1's height above floor, KH2's displacement through KH1's mover `0x140297710`
  (stepping down up to 24 onto lower floor on the ground, put back on the floor when below
  it), facing written to `+0x30..+0x3C` and `+0x150`, KH2's animation player for the pose
  (field variants on KH1's field slots). KH1 doesn't see Circle / Square while KH2 has him.
  Every movement ability at MAX for testing. Insert: KH2 controls / KH2 animations on KH1 /
  off / T-pose / idle / run. Cross still goes to KH1 (talk, chests, menus, KH1's attacks,
  which hand control back to KH1 until they end); KH2's combo system is next.
- Stage 8 result (the user + log): KH2 drives walk, run, dodge roll, quick run, aerial dodge
  and glide (300-500 units up off ledges); the jump played but didn't rise: KH1's mover snaps
  a grounded actor back onto the floor each frame, undoing the jump's first small rises.
  Fix (8b): only the sideways move goes through the mover; in the air (or rising) his height
  is KH2's, landing on the floor the mover reports (`+0x324`); on the ground the mover's
  height stands, stepping down onto a floor up to 24 below, else he falls. `+0x3D4` is
  rewritten for KH2's next step. The user's next requirement: attacks 1:1 with KH2.
- Phase 3 (combat parity) started; the user: movement looks great. Ghidra:
  - KH1's hit pipeline: `0x1402BD180` `(attack, target)` (an attack touched an actor: checks
    the target can be hit `0x14028FEE0`, queues a hit record, MP for the attacker from the
    attack data `+0x2C`, the attack's hit script) → `0x1402BD480` `(attack, hit-once bit,
    target)` takes a 0x40-byte record from the pools at `0x142DE6810` / `0x142DE7810` and
    links it on the target (`+0x3EC`, hit-once bits `+0x3E0`): attacker `+4`, damage `+8`,
    flags `+0xC`, knock-back direction `+0x10`, attack data `+0x2C`; the target's think pops
    it (`0x1402BCFB0`) and applies damage and reaction.
  - KH1's damage `0x1402BD8D0` `(attack, target, flags)`: power = attack data `+0x14` +
    attack `+0x2C`, critical (chance `+0x94`, bonus `+0x98`), against the target's defence
    (stats `+0x6C` → `+0x50`) through the table `0x142DE67C0`, min / max from the attack data
    (`+0x20` / `+0x24`) scaled by attack `+0xAC`, clamps from the target's stats (`+0xC0` /
    `+0xC2`), element resistance (stats `+0x78 + element * 4`, element = attack `+0x30` bits
    6-9). Attack objects: owner handle `+0x34`, attack data handle `+0x5C`, hit-once bit
    `+0x54`.
  - KH1's collision pass `0x1402B1A50` tests 0x40-byte entries (owner, attack object,
    shape at `+0x20`, overlap `0x1402B3D50`); attack vs body → `0x1402B1ED0` →
    `0x1402BD180`; attack vs attack → `0x1402B2A30` (clash, priority `+0x68`).
  - Battle mode: bit 0 of `0x142867CD8` (picks field / battle idle-walk-run in
    `0x1402C9FF0`). Actor iterator `0x14028F990` `(previous) -> next`. Actor kind byte `+6`
    (3 enemy AI, 4 / 5 / 9 / 10 objects), body radius `[[+0x134] +0x14] +0xC`.
- `kh1-hook` stage 9 (combat probe): set-motion hooks log the code putting Sora into
  non-movement slots (stack return addresses), the attack-touched-actor hook dumps Sora's
  attack records / targets' stats and keeps the last as a template, the actors are listed
  every 4 s. Fixed before shipping: `0x14029D630`'s third argument is a float (XMM2).
- `kh1-hook` stage 10 (combat parity, `win/combat.rs`), from the stage 9 log and Ghidra:
  - The Attack command `0x1402A5380` (command type 1 in `0x142D54440`) runs a row of the
    actor's attack table with `0x1402A4760` `(actor, row, dir)`; `0x1402A17F0` reads the row
    (`0x142D5D4B0[object id] + row * 0x14`: motion request `+0`, attack data
    `0x142D23740 + [+4]`, effect / sound bytes `+0xC..+0xE` → attack `+0xA8..+0xAA`, flag
    `0x400000`) and makes the attack object with `0x1402B10E0` `(actor, hit bit from
    0x1402B15D0, data)` (pool `0x142DB5110`, 0x40 × 0xC0; freed by `0x1402B13D0`).
  - Per frame `0x1402B22D0` builds the body entries: hittable actors (`0x14028FEE0`), team
    `(+0x374 >> 5) & 3`, parts at `[[+0x134] +0x18]` (0x14 bytes each, count byte `+6`,
    disabled bits `+0x3A8`), shape by `0x1402B3CB0` (type, radius, half height, centre `+0x10`).
    Contact `0x1402B1ED0` `(attack entry rcx, body entry rdx)` does the record, its push
    direction, the hit effect and the Keyblade hit sound (`0x140293FD0`).
  - While KH2 has Sora, a type-1 Attack command is swallowed and becomes KH2's attack press.
    KH2's swept attack shapes and hit records  test KH1's body parts;
    each effective hit is landed by a fresh attack object from Sora's own row (ground / air ×
    swing / finisher = KH1 motions 200 / 202 / 203 / 205), hit-once bit cleared during the
    call, then freed. `probe::hit` scales the record's damage by the KH2 row's power.
- `kh1-hook` stage 11: the user's stage 10 log: KH2's hits land (31-72 on the ground); the
  Attack command's air rows are 204 / 205 (203's first row, 0x4C58, hit for 95-144). Area
  scripts (`0x14028C550`, per-actor action list at `+0x440`, pool `0x142BA1E80`, 0x60 each,
  kind byte `+0x10`, next `+0x58`) walk Sora through doors / zone entries: KH2 lets go and
  `pose` shows KH1's own animation while the list is non-empty. Diagnostics for the
  Keyblade (invisible during KH2's attacks): its instance is combat state `+0x18`; where it
  is posed is logged with each KH2 state change.
- `kh1-hook` stage 12: the user: the game crashed going through a door (stage 11; no dump,
  last line KH2 retaking Sora between the door's turn and its walk, actor flags `0x1000000`
  set). Actor flag `0x1000000` (script commands 0x24 set / 0x23 clear) now also keeps KH2
  off (set from a door's turn to the end of its walk, and at a zone entry). `win/crash.rs`
  logs the first fatal exceptions (code, address, registers, return addresses). The
  Keyblade instance is never posed by the two skeleton passes; its own matrices are logged.
- `kh1-hook` stage 13: the crash note found the door crash: ours, in the stage 9 probe's
  set-motion logging (`probe::callers` scanned 0x800 bytes above its frame and ran off the
  top of the game thread's stack when a door script set Sora's exit motion). The scan now
  stops at an unreadable page. The Keyblade object (combat state `+0x18`) is no model
  instance (`[0x82, 0, 0, 9, handle, ...]`); its model instance is the handle at `+0x10`.
- `kh1-hook` stage 14: the user: doors work; the Keyblade is still invisible while
  attacking. Ruled out offline (`kh1-data` example `hand_check`): no KH2 attack motion
  degenerates or stretches the retargeted right hand or the 40 bones under it (101-127,
  280-292). The combat state's `+0x18` object is no model either: it is a table (count `+0xC`,
  0x20-byte entries from `+0x10`, looked up by `0x1401DDE80`), the hit-effect points. Probe:
  every small model (≤ 40 bones) posed near Sora is logged with where its placement matrix
  comes from (a bone of Sora's drawn matrices, if it points into them), at each KH2 state
  change and every 15 frames of an attack.
- `kh1-hook` stage 15: the stage 14 log: no small model is posed near Sora's hand at any
  time (none of the passes poses a Keyblade model). KH1 Sora's own skeleton carries the
  weapon: bone 280 under the right hand (grip, 4 units from it), 281-282 at the same point,
  283-292 a chain of short segments (flags 1) reaching 56 units (`kh1-data` example
  `bones_under`). The probe now logs bones 280 / 283 as KH1 posed them and as we drew them
  (position, determinant, distance from the hand). The weapon-object probe is gone.
- `kh1-hook` stage 16: the user's video (Traverse Town, no enemies): Sora's hands are always
  empty; the weapon bones are drawn in his hand (stage 15 log), so KH1 hides the Keyblade
  itself: KH1 shows it only in its battle motions (field idle / walk / run 0 / 2 / 5 carry
  none; KH1 brings it out for attacks). KH2's field / battle stance now follows KH1's battle
  mode (not KH1's slot), and while KH2's pose has the Keyblade out (`weapons_out`: battle,
  or an attack in the field) Sora's actor is kept in KH1's battle stance (`+0x374` bit
  `0x40000`, which `0x14029E050` uses to map a request to the field or battle slot) with
  the idle request replayed (`0x14029D630(actor, 0, 0.0)`); back to field when it is put
  away. Finished actions blend into idle over 14 frames (`RigAnim::queued_blend`).
- `kh1-hook` stage 17: stage 16's idle request never changed Sora's slot (stayed 0, redone
  every frame): `0x14029E050` maps a request to a slot through the motion table, and the
  stance bit only applies to actors with flag `0x4000` or no combat state, so not to Sora.
  Request bit `0x20000000` passes a slot through unmapped: the battle idle (slot 1) is now
  set directly while KH2 has the Keyblade out, the field idle (0) when it is put away.
  (The user: fetch `kh1-kh2.log` from the PC directly, never ask for it.)
- `kh1-hook` stage 18: stage 17's raw battle-idle slot took, but KH1 put the field slots
  back every frame and the Keyblade stayed hidden. The real switch (Ghidra): applying a
  motion (`0x14029DAB0`) draws the weapon for table entries of type 1 (`0x1402A0CA0(actor,
  1)`: actor flag `0x4000`, combat state `+0xC..+0xF` = ff ff ff 80) and puts it away for
  type 0 (`0x1402A10F0`: clears the flag); the flag also maps requests to their battle
  variants. The flag now follows KH2's pose (`weapons_out`). After an attack in the field,
  KH2 keeps its battle stance for 6 s (the user: combos ended in the empty-handed idle).
- `kh1-hook` stage 19 (retarget polish, the user's video: the Keyblade shows now):
  - Trouser leg snapping in falls: the thigh's roll flipped 180° within a half frame in
    falls, landings, jumps and several combo swings (`kh1-data` example `snap_check`). The
    knee's direction came from the knee's offset from the hip-ankle line, which lies along a
    raised thigh; it is now the hinge plane's (`bend`: thigh × shin), with the hips' forward
    swung along with the leg as the straight-leg fallback (`Pole::Knee`). Elbows use the same
    `bend`. Worst half-frame thigh turn: 180° → under 50°.
  - The shorts' four panel roots per leg (hip + 10 / 14 / 18 / 22) turn halfway between
    pelvis and thigh.
  - Shoes: the foot's roll is the model's up while the foot lies flat (`Pole::Up`), the shin
    only as it stands up, so soles stay flat when KH2's shins lean out.
  - The hood (root 79) turns with the chest's yaw and 30% of its lean; every bone no part
    turns now hangs as in the first frame of KH1's battle idle (slot 1, `set_rest`) instead
    of the T-pose (hood, belt chain, cloth joints, face, the Keyblade's grip and chain).
- `kh1-hook` stage 20 (the user's video of stage 19: the belt chain floated out in a loop,
  shins showed through the shorts in the battle idle, odd thumbs in the field idle, the hood
  could be lower, Sora briefly invisible, feet clip stair steps):
  - KH1 rest pose only for the hood's cloth joints (80-84) and the Keyblade (280-292); the
    belt chain (10-15) and the rest keep the bind (the battle idle's chain swings out).
  - Shorts panels follow the thigh again (the snap was the thigh roll, fixed in stage 19).
  - Hood root: chest yaw only, tipped down 0.45 rad (tip 45° below the neck).
  - Thumbs mapped (KH1 207-211 / 103-107 on KH2's thumb segment 89→92 / 181→184).
  - The skeleton pass's world matrix, once confirmed, places every later Sora at once (it
    was re-learnt per zone, placing him by his bones meanwhile: the brief invisibility).
  - Open: feet on stairs (KH1's ground IK is replaced by our pose; needs our own foot IK).
- `kh1-hook` stage 21 (the user: thumbs still odd, feet through stairs, the chain through
  the shorts, fit the jacket's lower part to KH2's motions):
  - Thumbs (103-107 / 207-211) take KH1's own pose: the battle idle's grip while KH2 has the
    Keyblade out (`takeover::GRIP`), the field idle's (slot 0) open hand otherwise
    (`Retarget::set_rest_open`, `pose_grip`). The KH2 thumb mapping is gone.
  - Belt chain root (10) turns 80% with the +x thigh; the jacket's back hem (72) 35% with the
    chest, the rest with the hips.
  - `win/feet.rs`: foot IK while KH2 has Sora on the ground. KH1's floor ray `0x1401910E0`
    (the ground-IK pass's query: start `+0`, 1.0 at `+0xC`, length `+0x14`, 1.0 at `+0x1C`; hit
    `+0x20..+0x28`, hit id i16 `+0x30`, -1 none; KH1 world y-down) under each sole (bones
    22 / 48); the body drops for a planted foot's lower floor (≤ 30), each leg bends (thigh
    17 / 43, knee 18 / 44, ankle 19 / 45) to put a planted sole on its floor or lift a raised
    one out of a step; eased (35% a frame).
- `kh1-hook` stage 22 (the user: the chain still sits oddly, the thumbs never changed, the
  jacket must mesh with KH2's motions). Measured offline before shipping (`kh1-data`
  examples `thumb_check`, `clip_check`, `kh1_follow`, `snap_check`):
  - Thumbs: KH1's own motions leave the thumbs at their bind (so no KH1 rest pose changed
    them). They are now aimed along KH2's thumb (89→92 / 181→184) with the part's rest frame
    measured on KH1's bind (`layout`'s `kh1_rest`): KH1's thumb lies exactly as KH2's
    (0° apart in every motion checked).
  - KH1 animates its cloth (belt chain, hem) by hand per motion (no blend of hips and thighs
    fits it: 20-140° residuals), so: the belt chain (10) hangs with the hips' yaw only; the
    jacket hem's nine chains (72; 248-276) turn with the hips (KH2 parents its own hem to the
    pelvis), front panels a little with their thigh; and `keep_clear` swings any cloth chain
    about its root so no joint comes nearer a thigh (hip → knee) than in the bind (≤ 14).
    Hanging and pushed cloth move at most 0.35 rad a frame (no snaps when the hips' yaw
    flips in somersaults).
- `kh1-hook` stage 23 (the user: in the battle idle the chain clips through the shorts and
  the hood through Sora):
  - The belt chain (10-15) is carried rigidly by the +x thigh (43) as in the bind
    (`ATTACHED`, `attach`): its distance to the outer shorts panel (57→59) is the bind's in
    every motion checked (a free-hanging chain fell inside the shorts when the battle
    stance spread the legs).
  - The hood chain (79: 80-84) is kept off the back (71→78) and the head (78→151) by
    `keep_clear` (up to 30), so the hunched battle stance can't tip the back into it.
- `kh1-hook` stage 24 (the user's stills: the back of the chain lies in the back of the
  shorts in every stance; the hood clips wildly in the battle idle):
  - Hood: no more hanging or pushing. Root and joints (79-84) take KH1's battle-idle relation
    to the neck base (78, on the chest part), so the hood lies on the back as in KH1 and
    moves with the chest (it can't meet the back). Tip 2-8 above the neck in KH2's stances.
  - Chain: still carried by the +x thigh; its lower joints (14, 15) also kept 4 further off
    that thigh than in the bind (constant: 13.8 / 20.1 from the thigh axis in every motion
    checked). The chain mesh's back part is presumably skinned with the hips (we don't read
    KH1's skin weights yet); if it still clips, reading the mdls mesh weights is next.
- `kh1-hook` stage 25 (the user: the hood is at 90° again; leave the chain; shoes spin round
  in the glide):
  - Hood: still carried by the chest in KH1's battle-idle relation, now folded down 0.7 rad
    about the level axis square to it (`HOOD_FOLD` in `set_rest`): tip 7 below the neck
    base, its joints 7-13 off the spine axis, constant in every motion.
  - Feet: `Pole::Foot` replaces the up rule: the foot rolls with the shin's hinge axis (the
    shin part's x, frames now computed in order, `frames`), turning to the model's up only
    while the foot is flat and its ankle low (below 24, none above 40, KH2 units). The up
    rule alone had turned the glide's pointed, sole-up feet over. `foot_check`: glide
    (205-207) shoes within 9° of KH2's feet.
- `kh1-hook` stage 26 (phase 4, combat parity, finishing):
  - Lock-on: KH1's lock state `0x142DB3F10` (non-zero while R1 holds a target) and target
    handle `0x142DB3F18` (`0x1402912F0`, the command menu, aims every command at it). KH2's
    controller locks the same enemy (`Character::set_lock`), so swings home in on it.
  - Finishers land with a KH1 finishing row whose attack data differs from the swing's
    (KH1's first 202 row shares the swing's data); every distinct attack data's first words
    are logged for picking knockback by its fields next.
- `kh1-hook` stage 27 (the user: enemies like the Large Body must repel hits from the front;
  some enemies' parts take no damage and bounce the Keyblade off):
  - KH1's guards are attack objects of priority 100 (`+0x68`; the clash `0x1402B2A30` repels
    anything meeting one), directional when type bits `0xE0000` are 1 or 3 (`0x1402B24B0`:
    Sora within the arc whose cosine is the data's `+0x64` around the owner's facing turned
    by the data's `+0x60`). `combat::guards` scans the attack pool (`0x142DB5110`, 0x40 ×
    0xC0; in use `0x800`, active `0xC`, motion-bound type 2 only while the owner plays it)
    each frame; a KH2 swing meeting one is repelled (`Character::register_wall_hit`, KH2's
    repel) and hits nothing more, with KH1's clash spark (`0x1402AC4C0(*0x142D67400, 10,
    20000, pos)`). Guards seen are logged once per owner.
  - Finishers: KH1's combo finishers are motions 203 / 206 with reaction-kind-5 data
    (0x4C58; the data's third word is the reaction kind, 3 a flinch, 5 knock-back).
  - Lock-on confirmed in the user's log (KH2 locks KH1's R1 target).
- `kh1-hook` stage 28 (Phase 5, taking hits): `win/hurt.rs` hooks KH1's hit pass `0x140299A40`.
  Before it runs for Sora under KH2, each ready hit record (queue at actor `+0x3EC`, next at
  `+0x30`, delay bits 12..15 of `+0xC`) is decided with KH2's last frame: invulnerable
  (dodge roll) → evaded, guard state / guard windows with the attacker in front (Guard Plus:
  anywhere) → KH2 guard repel + KH1 clash spark; both mark the record no-effect (`+0` bit
  0x20, as KH1's own guard does). Taken hits keep KH1's damage pass (Second Chance, MP,
  HP bar, dying) with the reaction kind cleared, then KH2 plays the reaction (3 / 4 flinch,
  5 blow) and the hurt voice `0x14028E060(actor, 0|1, 0)`, rumble `0x14028D740`, journal
  count `0x14028A4D0(0xD)` and command clear KH1 would have. Damage types 9..12 (drains,
  heals) are left to KH1. HP gone / stats flag 4: `takeover` releases Sora to KH1.
- `kh1-hook` stage 29 (Phase 6 / 7 groundwork):
  - Platform carry and push: KH1's physics step adds, before its move, the carrying
    platform's motion (`+0x200` while `+0x160` bit 4) and the frame's push (`+0x120`, zeroed
    after), and clears the touched-objects mask (`+0x90`). KH2's moves now do the same
    (`takeover::carried`, logged the first 40 times it is non-zero). Before, `+0x120` was never
    cleared while KH2 had Sora.
  - KH1's movement state (`+0x70`) is in the hand-off logs (for Phase 6/8 hand-offs).
  - Phase 7: `growth.rs` (unit tested) maps KH1's menu to KH2's abilities: save block
    `0x142DE9D60` (`*0x1428695A0`), Sora's level byte `+4`, abilities `+0x44` (48 bytes,
    bit 7 = not equipped, 0 ends the list as `0x140282A70` counts), shared abilities `+0x599`.
    KH1's own abilities map directly (High Jump levels, Glide + Superglide, Guard, Dodge Roll,
    Combo Plus, Air Combo Plus, the FM modifiers to KH2's nearest); KH2-only ones come with
    Sora's level (`LEVELS`). `win/menu.rs` reads `kh1-kh2.ini` (`abilities = test | kh1 |
    all`, default `test`, the old testing set) and logs what KH1's menu gives every time it
    changes.
- `kh1-hook` stage 30: `win/settings.rs` reads `kh1-kh2.ini` once (written with defaults when
  missing): `abilities` (test | kh1 | all), `hurt` (kh2 | kh1), `carry` (on | off),
  `ceilings` (on | off), so each feature not yet tried in game can be switched off without a
  build. Ceilings: while KH2's displacement rises, the move goes through KH1's mover in 3D
  with Sora's movement state set to 5 (falling) for the call only: `0x140297F20` builds the
  collision query's mode from the state (air mode 0x10 vs ground 4 for the player's team;
  the ground mode is what snapped rises back to the floor in stage 8). Less than 49% of the
  rise done: KH2's `hit_ceiling` (logged with the collision flags `+0x2D0`; bit 3 is what
  KH1's own jump `0x1402AD260` treats as a ceiling).
- `kh1-hook` stage 31 (Phase 6, KH1-only moves): KH1's jump runs, in its per-state input
  step (`0x1402ADA90`, the state table's third handler), checks that hand Sora to KH1's own
  air moves: `0x1402C5710` vines / climbable surfaces (state 10, collision id `+0x2DE`),
  `0x1402C68A0` ledges (state 13, from the collision's contact `+0x2F0` / `+0x300` and the
  stick), `0x1402C7E00` bars (skipped while `0x14298D3A8`), `0x1402C17B0` hang points on
  other actors (body part type 3), `0x1402BF410` swimming (state 17, water level `+0x2D4`).
  While KH2 has him falling (Air / Jump, stick held) they are tried in the same order (ini
  `grabs`); one that takes him sets `HANDOFF`: KH1 keeps him until its state is 0 / 1 and its
  motion slot a movement one again, then KH2's controller starts over standing (`FRESH`).
- `kh1-hook` stage 32 (review fixes for 28–31): KH1's fall state 3 (not 5) for KH2's rises
  and the grab checks: its collision has flag 1 (no floor snap) and mode 4 like KH1's own jump,
  its input step has the grab checks, its wall-solver extra only clears `+0x3C4`. Grabs refresh
  KH1's contacts with a standing-still move in that state first and keep the state only if a
  check took him. `HANDOFF` lasts at least 10 frames. Any release marks `FRESH` (KH2 restarts
  standing when it has him again), and the in-air exception uses `airborne` only while KH2
  has him. `hurt`: KH2 reacts only to kinds 3–5 (others stay KH1's), never restarts a blow nor
  a flinch with a flinch, plays KH2's recoil (not its guard) for attack clashes 6 / 7, and
  passes the pass's rax back. Rises take KH1's mover's height (`y = got`).
- `kh1-hook` stage 33 (Phase 8, KH1's commands stay KH1's): hook on KH1's command step
  `0x1402A5380 (actor, dir)` (called from the think step `0x1402A4090` while a command is
  pending at `0x142D54440`). Attack (type 1) still becomes KH2's attack via `exec`; types
  that only touch the party or the menu (`0x1402A1470`'s switch: 0x1D 0x1E 0x46 0x51 0x54
  0x64 → `+0x98`'s actor; 0x4B.. 0x60 → menu) pass; any other (magic is 2, spell in bits
  8..23) hands Sora to KH1 (`takeover::hand_to_kh1`, the `HANDOFF` rules). KH1 picks the
  command's version by its movement state (0 ground → `0x1402A1C20`; 2 jump → the air cast);
  in KH2's air he is first put into KH1's jump state falling from where he is, as KH1's
  ceiling bonk does (`0x1402AF7D0(actor, 2)`, `+0x3B0` / `+0x3B8` / `+0x3BC` 0, `+0x3B4` his
  y, `+0x3C0` velocity 0, `+0x3D0` 0, flag 0x10000 off). Ini `commands`.
- `kh1-hook` stage 34 (Phase 9 feel): KH1's combo row (0x14 bytes) also carries what its
  motion's start plays (`0x1402A17F0`): +2 the weapon trail (`0x1402C0160(actor, n)` asks for
  it in the combat state, `+0x38` = n | 0x40000000; `0x1402BFD70` makes it while actor
  `+0x1CC` bit 0x800000, which the attack motions' triggers set over the swing, and ends it
  when the bit clears), +3 an effect on Sora (`0x1402ABB60(*(cs+0x18), n, actor, 0x30)`),
  +0xF a voice (`0x14028E060(actor, n, 0)`). Each KH2 attack motion start plays those of the
  KH1 row of the same place in KH1's combo (200 / 201 / 202, finisher 203; air 204 / 205,
  finisher 206), and the trail bit follows KH2's hit windows (ini `swing_fx`; whether the
  trail was made is logged for the first swings).
- `kh1-hook` stage 35 (review fixes for 33–34): the trail flag is cleared whenever KH1 has
  Sora (any physics frame KH2 doesn't take, and before a KH1 command); a command hands Sora to
  KH1 only if KH1 changed his state or motion request (`+0x164`), else the in-air jump state
  is undone and KH2 keeps him; `exec` and `command` act only while KH2 has him and he is not
  handed to KH1 (`takeover::kh2_has`); a stale Attack press is dropped when KH2 starts over;
  a repel's recoil (KH2 motions 41 / 43) is not a swing start; the swing rows are cached per
  model and never empty.
- `kh1-hook` stage 36 (Phase 7, KH2's abilities in KH1's menu): KH1's ability table is
  `btltbl.bin`'s (loaded to `0x142D23740` by `0x14028D840`), entries of 12 bytes at
  `*0x142D37748` (static `0x142D26B18`), 65 of them: +0 AP, +1 kind, +2 menu order (u16; the
  menu sorts lists by it, `0x1402836C0`), +4 name handle, +8 help handle (from the remaster's
  `AbilityName.bin` / `AbilityHelp.bin`, made handles with `0x14038B080`, which registers any
  pointer's 32 MB window in the handle table `0x142EE4730`). `0x14028D760(id)` = table +
  (id - 1) * 12 is every reader (menu name `0x1402E5D40`, help `0x1402E5D10`, AP used
  `0x1402E62D0`, the message placeholder in `0x1401746D0`). `win/menu.rs` copies it into an
  80-entry table with ids 66..77 (template entry, KH2 AP, our text in KH1's codes), points
  `0x142D37748` at it (again after `0x14028D840`, hooked), and in `abilities = kh1` adds them
  to Sora's lists at their level (`growth::MENU`; shared ones on, the rest off); other modes
  take them back out. Final Mix's Combo Master (65) maps to KH2's. Commands now count as
  KH1's when the script-held flag or the motion slot changes too (spells set the flag).
- `kh1-hook` stage 37 (the user: no animation for a jump cast; the Square shortcut didn't cast;
  check moves don't repeat):
  - A command counts as KH1's when KH1 sets any motion on Sora during it (`probe`'s
    set-motion hooks count them: `SORA_MOTIONS`); the air cast (`0x14029D7F0(actor, 0x83 +
    spell)`) changed none of state / request / flag / slot at that point, so KH2 kept him and
    hid the cast's motion.
  - KH1's shortcuts are L1 + a face button: with L1 held, Circle and Square reach KH1 and are
    not KH2's jump / action (a button held past L1's release stays KH1's until let go).
  - `kh1-data/examples/combo_mix.rs` runs KH2's controller with the user's menu (Combo Plus x4,
    Air Combo Plus x2, Combo Master, the nine mapped modifiers) against near / far /
    surrounding / raised foes: no move follows itself; ground combos mix 151 / 152 / 153 with
    Slapshot, Flash Step, Slide Dash, Vicinity Break and finish with Guard Break / 155 / 154,
    air combos alternate 181 / 182 (as KH2 does) and finish with Aerial Finish.
  - A full ability list (48, KH1's most) is reported once per ability, not every second.
- `kh1-hook` stage 38 (the user: KH2 abilities have priority): when Sora's list (48) is full,
  `growth::make_room` picks a KH1 ability that is off to give up its place: first a spare copy
  of one he has two or more of (most copies first), then, in `MAKE_ROOM` order, Encounter
  Plus, EXP Zero, Cheer, Scan, Evolution, Jackpot, Lucky Strike, Treasure Magnet, Second Wind,
  Tech Boost, Leaf Bracer, MP Haste, MP Rage, Berserk, Critical Plus, Second Chance. Never an
  equipped one, ours, Guard, Dodge Roll, the specials or the modifiers. Each removal is logged
  and appended to `kh1-kh2-removed.txt` next to the game (unit-tested on the user's list).
- `kh1-hook` stage 39 (the user: keep Encounter Plus; Magnet Burst doesn't pull; where is
  Explosion?):
  - Encounter Plus is off `MAKE_ROOM`; abilities `kh1-kh2-removed.txt` lists that may no
    longer be given up are given back once a run (off), another making room if needed.
  - Magnet (ATKP reaction category 5, Magnet Burst): an enemy it strikes is caught (orbit
    radius between the attack's two knockback values, swirl its next value) and, while the
    swing lasts, drawn to the pulling hitbox's centre with the exe's suction step (toward the
    radius at up to 20 a frame plus the swirl, as `character_core::npc`) through KH1's per-frame
    push `+0x120` (its physics step adds it to the move and clears it, enemies included).
  - KH1 abilities that are KH2 moves here show their KH2 name and help in the menu
    (`growth::RENAMED`: Vortex → Flash Step, Counterattack → Counterguard, Blitz → Finishing
    Leap, Sliding Dash → Slide Dash, Hurricane Blast → Explosion, Ripple Drive → Vicinity
    Break, Stun Impact → Guard Break, Gravity Break → Aerial Finish); ids unchanged.
- `kh1-hook` stage 40 (the user: Magnet Burst's and Explosion's exact KH2 sounds and effects?):
  `win/kh2sound.rs`, the KH2 sound player (`docs/kh2-sound.md`): once KH2 Sora is loaded,
  the sounds of the effect elements the chosen motions start (`EventKind::Effect`, PAX "p_ex"
  in `obj/P_EX100.a.us`) are read from the user's KH2 install (`se/seNNN.win32.scd`, Microsoft
  ADPCM) and played through `waveOut` when KH2's clip starts the element. Ini `kh2_sounds`
  (`magnet_explosion` default: 166, 197; `all`; `off`), `kh2_volume`. KH2's visual effects
  (PAX/DPX particle systems) are not drawn yet: that needs our own particle renderer inside
  KH1's Direct3D 11 frame.
- `kh1-hook` stage 41 (the user: the KH2 sounds don't sound right; dig into the KH2 install):
  `docs/kh2-sound.md` "Sound ids are id records": `Scd::sound_id` resolves a sound id through
  the bank's id records (it was played as a stream number: wrong sounds), with
  their volume; Ogg Vorbis voice streams decode (`lewton`). `EventKind::Sound` (frame trigger
  8, weapon bank 500 = the Kingdom Key's) and `EventKind::Voice` (13 / 14, VTBL) join the
  effect elements; `kh2sound` plays all three, and KH1's swing voice stays quiet on those
  moves. Research tools: `kh1-data/examples/move_sounds.rs` (renders a move's KH2 timeline to
  WAV), `atkp_sounds.rs`; the static Linux CLI runs on the user's PC VM
  (`local/tools/kh2-combat-linux-3 extract --backup --game ...`).
- `kh1-hook` stage 42 (the user: every KH2 sound is right but the voice; keep KH1's voice):
  `kh2_voice` in `kh1-kh2.ini`, off by default: `kh2sound` loads no VTBL voices and
  `covers()` is false, so KH1's swing voice plays on Magnet Burst and Explosion with KH2's
  sounds and effect elements. `kh2_voice = on` brings KH2's battle voice back.
- `kh1-hook` stage 43 (the user: give Sora more AP to make up for the KH2 abilities): KH1's
  party stats `0x1402A2FA0` `(member, out)` fill `0x142E94BC0 + member * 0x100` from the save
  (max AP `+0xE4` = member `+9` plus the accessories' `+4`, at most 100; `0x1402E6220` is AP
  left = that minus the equipped abilities' AP, `0x1402E6120` reads it). `win/menu.rs` hooks
  it: Sora's max AP gets `growth::bonus_ap(level)`, the AP of every `growth::MENU` ability
  learned by then (27 by 50). Nothing is written to the save. `bonus_ap` in the ini.
- `kh1-hook` stage 44 (the user: KH2's real visual effects for Magnet Burst and Explosion):
  - `docs/kh2-effects.md` + `kh2_formats::dpd`: KH2's effect machine (PAX elements, 30 Hz
    effect ticks, emitters / programs / keys, MegaBirth sprites, ribbons, models, textures
    with the palette swizzle, `effect/texcommon.dpd` for VRAM addresses below 0x3000).
  - New crate `kh2-fx`: plays elements and outputs world-space triangles per batch (texture,
    GS blend, depth test, depth offset); `examples/fx_render.rs` renders them to PNGs.
  - `docs/kh1-render.md`: KH1 renders with **Direct3D 12** (D3D11 only decodes movies). Renderer
    `0x1422BB580` (device `+0x2E0`, frame buffers, constant shadows), View `0x142E9DE60`, P at
    the projection struct `+0x80`, KH1's clip formula, the world-model pass `0x1401EADD0`.
  - `win/fx12.rs`: our D3D12 drawing recorded into KH1's current list after the world pass, on
    its bound frame buffer (RTV / DSV by id, its MSAA count), own root signature (40 root
    constants + a root SRV raw buffer of RGBA8 textures, bilinear wrap in the shader) and
    PSOs per blend mode; KH1's root signature, heaps and dirty state restored with
    `0x1401087B0` / `0x140102980` (KH1 re-binds topology and vertex buffer per draw itself,
    `0x1400FBA20`). `win/kh2fx.rs`: loads `P_EX100.a.us` + texcommon from the KH2 install,
    starts elements on `EventKind::Effect`, ends motion-bound ones on a motion change, steps
    per KH2 controller frame, bones from `pose::kh2_pose`. Ini `kh2_effects`, `fx_test`.
- `kh1-hook` stage 45 (the user: the game crashed at Explosion's effect): KH1's current list is
  usually a **bundle** (`0x1400FE6E0` creates its allocator and list with type 1;
  `0x1400F7290` closes it and runs it with `ExecuteBundle` in the direct list at command
  buffer `+8`). Our `OMSetRenderTargets` is not allowed in a bundle, so its `Close` failed
  (`E_INVALIDARG`) and KH1's `ExecuteBundle` of the broken bundle crashed. `fx12` now checks
  the list type (`GetType`) and sets targets only in a direct list (a bundle inherits them),
  then hands KH1 a fresh list with its own mid-scene sequence (`0x1400F7300(R, 0)`,
  `0x140109360(R, 1, 0)`, as in `0x1400FBBA0`) instead of `0x1401087B0` + `0x140102980`.
- `kh1-hook` stage 46 (the user: casting Thunder crashed the game): the hit pass `0x140299A40`
  takes **two** arguments, `(actor, out vector)` (rdx, kept in r13 and passed to the
  reactions as r8; the knock-back reaction `0x14029AD30` writes the actor's position there).
  `win/hurt.rs` hooked it as `(actor)` and called KH1's with a garbage rdx; harmless while
  KH2 had Sora (KH1's reaction kind cleared), it crashed when a hit landed while KH1 had him
  (a spell cast). The hook now takes and passes both.
- `kh1-hook` stage 47 (the user's ability levels): High Jump (one) = High Jump MAX; Glide =
  Glide LV3, Superglide = Glide MAX; Quick Run and Aerial Dodge LV3 when learned (their level
  growth dropped; the dodge roll keeps its own). Sora keeps one High Jump / Glide / Superglide
  (`growth::one_only`, an equipped copy kept): spares are taken out of the shared list in
  `abilities = kh1` and noted in `kh1-kh2-removed.txt` (without "(id", so they are not given
  back).
- `kh1-hook` stage 48 (the user): High Jump is High Jump LV3 (not MAX).
- `kh1-hook` stage 49 (hook audit, after stage 46's crash): for every hooked function, the
  argument registers it reads before writing (rcx / rdx / r8 / r9, xmm0..3) and whether its
  callers use rax / xmm0 after the call (a Ghidra `CallUse.java` over every call site), and the
  same for every KH1 function we call. Found: `0x14029D7F0` and `0x14029D8A0` (set motion)
  take a float third argument in XMM2 (stored at actor `+0x184`) and their result is used by
  callers; `0x140392710` (skeleton pass with ground IK) takes a float third argument. The hooks
  declared those as integers, so KH1 got whatever XMM2 held: now `f32`. Everything else matches
  (`0x1402A4760` 3 ints, `0x1402A5380` 2, `0x140299A40` 2 (46), `0x1401EADD0` /
  `0x14028D840` none, `0x1402A2FA0` 2, `0x140391E20` (int, int, float, int), `0x14029CF10`
  (int, float), `0x140112C70` 3, `0x14029D630` (int, int, float), `0x1402BD180` 2 → used
  result, `0x140298230` 1).
- `kh1-hook` stage 50 (the user: softlocked in the Dive to the Heart's attack tutorial; the
  Dream Sword held backwards; Sora moving while KH1 holds him still; without Combo Master one
  swing only):
  - KH1's attack step (`0x1402A4760`) counts each attack in the save's tallies
    (`0x14028A4D0(0x13)`: `+0x848 + id * 2`, capped 9999); event scripts read the tallies
    (`0x14028A4B0`, an event opcode at `0x1401ADF??`). KH2's swings now count too.
  - KH1's player control step `0x1402A4090 (actor, move vector)` (from the think step
    `0x1402A1170`, skipped when actor `+0x444` bit 1 is set) reads the pad and starts
    commands only when `0x1402956E0` says he can act (flag 4, `+0x88` bit 0x20, `+0x444`
    bit 1). Hooked: KH2 gets no input on a frame it didn't run with him able to act.
  - The weapon's grip bone 280: when KH1's own grip (relative to its parent, read from KH1's
    pose while it shows slots 0-7) is turned more than 1 rad from the battle idle's, it is
    used (`Retarget::set_grip`): the Dream Sword is held another way than the Keyblade.
  - `Abilities::one_swing_on_miss` (character-core, ours, set in `abilities = kh1`): without
    Combo Master a swing that hasn't connected doesn't chain.
- `kh1-hook` stage 51 (the user: at the start of some cutscenes Sora slides and snaps back;
  the Dream Shield is held backwards): held still on the ground (KH1's control step skipped
  or him unable to act) for 2 frames, KH1 has him (KH2's run slide or lunge no longer moves
  him while the event sets up), KH2 starts over afterwards. Logged once per weapon: Sora's
  weapon (save `+0x36`) and KH1's grip of it against the Keyblade's rest grip (for the
  shield's fix).
- `kh1-hook` stage 52 (the user: the Dream Shield's front faces Sora): ini `flip_weapons`
  (KH1 item ids, default 0x53, taken to be the Dream Shield: KH1's weapon items run Kingdom
  Key 0x51, Dream Sword, Shield, Rod; unconfirmed, the weapon line in the log gives the id):
  the grip bone's subtree is turned half round about the forearm at the hand
  (`pose::turn_weapon`).
- `kh1-hook` stage 53 (the user's video: with the Dream Shield, KH1's idle and attacks
  showed, a choppy cut out of a run, no attack while walking): the shield has its own
  moveset, whose motions our slot table (P_EX100's signatures) doesn't know, and an unknown
  slot was taken for KH1's own action (KH1 had him). Now only a known slot past 7 is; KH1's
  move hand-back takes an unknown motion as standing when KH1's last request was a slot up to
  7 (`kh1_moving`). Confirmed in the log: the Dream Shield is item 0x53 (`flip_weapons`'s
  default). The live-grip switch of stage 50 is gone (KH1's grip moves with its motions; it
  flapped between the two every few frames); its log line stays.
- `kh1-hook` stage 54 (the user: lock-on on Darkside's hand still sent Sora at his body;
  Wakka's ball can't be hit back; the Dream Shield still faced Sora; KH1's own moves (ladder)
  snapped in; items from chests were Archipelago items):
  - `docs/kh1-lockon.md`: KH1 locks a (handle, lock point) pair: point index at
    `0x142DB3F1C`; `0x1402AE330 (actor, point, out vec4) -> radius` is the point's world
    position (lock points: collision record `+7` count / `+0x1C` list, 0x14 each, mask actor
    `+0x3AC`). KH2's target for the locked actor is now that point.
  - `docs/kh1-reflect.md`: shots like Wakka's ball are free-standing attack objects of the
    pool (`+0x30` bit 0x400); a swing turns them back through the attack-vs-attack meeting
    `0x1402B2890 (entry, entry)` (`0x1402B2A30` both ways: team swapped, Sora the owner, flag
    0x1000000). `combat::shots` / `meet_shot`: KH2's swept swing against them, once a swing,
    with a KH1 row's attack object as the swing side.
  - The Dream Shield (`flip_weapons`): held with KH1's own wrist (the grip's parent bone
    relative to its parent, from KH1's pose each frame it poses him), carrying the fingers and
    the weapon; KH2's arm swings as before. (The half-turn of stage 52 is gone.)
  - KH1's own animation blends in from the last pose we drew over 0.2 s (`KH1_BLEND`).
  - The user's KH1 `Image/en/kh1_first` was a randomizer-patched package (Feb 2025: 26 files,
    Destiny Islands / Dive / Disney Castle `.ard`s, `UK_treasure.ev`, ...): restored from
    `BackupImage/`, the patched pair kept as `*.ap-patched`.
- `kh1-hook` stage 55 (Wakka's ball still not turned back; nothing in the log): the swing now
  meets every live enemy attack of the pool but guards (KH1's pass meets every pair; the
  meeting decides), and each one near Sora is logged once by slot and kind ("enemy attack
  near Sora: ...") to tell what the ball is.
- `kh1-hook` stage 56 (diagnostics for the user's T-pose arms, seen on the ladder and at
  Darkside): logs when KH1's own pass leaves both hands where the bind has them relative to
  the chest (with the pass, slot, the ground-IK pass's float argument (a blend weight, used
  by `0x140392710`'s lerps) and mode), and when the pose we draw does.
- `kh1-hook` stage 57 (the user's ladder video: "T-pose" arms dropping off the ladder): not a
  T-pose but KH2's battle fall (motion 4, arms flung up and out: hands at head height ±55
  apart in `retarget_check`) drawn for KH1's own fall (slot 10) while KH1 had him. KH1's own
  jump / fall / landing (slots 8-13) now show KH1's own animation while KH1 has him (KH2's
  jumps are KH2's as before). The T-pose detector of stage 56 is gone (its chest bone was
  three parents up from the hand, still in the arm: it fired in every idle).
- `kh1-hook` stage 58 (the user: walking into a ladder, the walk froze before the climb):
  `pose` kept drawing KH2's last pose for 0.15 s after KH1 took him (`takeover::POSE`'s
  freshness), then blended from it: a frozen walk for a third of a second. Taking him to KH1
  now drops that pose (`takeover::drop_pose`), and the 0.2 s blend into KH1's own animation is
  eased out (KH1's motion shows at once, our last pose fades).
- `kh1-hook` stage 59 (the user: can't land on boxes; Sora hung over a box in KH2's fall,
  arms out): the map floor point `+0x320` doesn't know objects, and KH2's falls set his height
  against it alone. Falls now go through KH1's mover in its fall state too (as rises did), which
  stops him on objects; a fall stopped short (less than half of it happened) lands him there
  (`Driver::on_top`), and on the ground he stays on top while the ground mover keeps him at
  that height (else the usual floor step-down / walk-off).

### 2026-10-16: Stage 60 (fewer clinks, wall clinks, box tops)
- **Enemy attacks a swing meets** follow KH1's own pass (`0x1402B2610` / `0x1402B1A50`): motion-tied
  attacks (activity 2) only inside their owner's hit window, `0xFFF` latching skipped, word bit
  0x400 and spent 0x80 attacks left out, the pairing table `0x1405299B8` checked both ways. The
  user: endless clinks against the enemies' resting attack shapes.
- **Walls:** each live KH2 swing casts KH1's background ray (`0x140191520`) from last frame's
  attack-shape centres to this frame's; a map face whose normal is mostly level (a wall) repels
  the swing (`register_wall_hit`, KH1's clang). Floors, ceilings and objects don't.
- **Box tops:** KH1's object-top finder `0x1402B3950 (actor, move) -> object` (types 2 / 3
  collision shapes of standable objects) is run in KH1's air state 2 (it skips state 3's class)
  from where a fall started, and each grounded frame a step down; he lands / stands on the top
  and `+0x398` is set (`0x140295C60`). The mover in our states never found them.


### 2026-10-16: Stage 61 (aim only at KH1's target)
- KH2's controller gets only KH1's current target (R1 lock, state 3, or the small cursor's soft
  target, states 1/2/4) in its target list, so swings neither turn toward nor home in on any
  other enemy or lock point; with none, a swing goes where the stick points. Ini `auto_aim = on`
  brings back KH2's own nearest-in-reach pick. The user: the automatic aim was too heavy.

### 2026-10-16: Stage 62 (wall clinks off by default)
- The user changed his mind: swings pass through walls again. The wall ray stays behind ini
  `wall_clink = on` (default off).

### 2026-10-16: Stage 63 (Quick Run LV2 later, both with AP, "New Ability" prompts)
- Quick Run: LV2, learned at level 12 (was 7); Quick Run (2 AP) and Aerial Dodge (3 AP) are in
  Sora's own list now, off until equipped, and their AP joins `bonus_ap`. Saves from earlier
  builds have them moved out of the shared list (one that was on stays on).
- Learning a KH2 ability shows a box in KH1's level-up prompt style ("New Ability", its name,
  its AP), through KH1's prompt boxes `0x14283BD90` / text `0x142DC3068` the way the
  Archipelago KH1 client shows items (gaithern's KH-1FM-AP-LUA, MIT, `show_prompt`); queued
  until Sora's box is free (a level-up's own box goes first).

### 2026-10-16: Stage 64 (walk out of the dodge roll)
- From the roll's fall-check event (frame 34 for Sora) the stick ends the roll and Sora walks /
  runs at once, as out of a landing; before, only a button did, so he stood still until the
  motion ended (the user: too much of a freeze after the roll).

### 2026-10-16: Stage 65 (New Ability box: the name only)
- The "New Ability" box shows the ability's name alone (one line), the user's choice.

### 2026-10-16: Stage 66 (KH2's field camera, Camera Style in KH1's Config menu)
- Research: `docs/kh1-camera.md` (KH1's field camera module, its mode table and state) and
  `docs/kh1-config-menu.md` (the Config menu's window, tables, text lookup, storage).
- **`win/camera.rs`:** KH1's camera mode table (`0x140507AD0`) gets ours for mode 1 (follow)
  and mode 3 (lock-on). Each frame `kh2_camera::Camera` (KH2's free follow / lock-on framing,
  wall pull-in with KH1's camera ray `0x140191640`) writes KH1's internal eye / target, with
  KH1's yaw / pitch / distance kept in step and its own timers zeroed; KH1 still blends to and
  from its other cameras, runs its climbing / ledge cameras (`0x1401E3320`), events and area
  cameras, and builds the View. Right stick: x turns (KH1's left/right option's sign), y zooms;
  R3 resets; Select is KH1's first person; KH1's Auto / Manual option is KH2's Auto. KH1 world
  ↔ camera: y flipped and divided by `camera_scale` (0.9).
- **`win/config.rs`:** a ninth Config row "Camera Style: KH1 / KH2". The window definition's
  row counts go to 9; its update is wrapped (our row's left / right / confirm switch the
  camera and are hidden from KH1's update; the row is drawn with KH1's panel calls; rows 25
  apart during the call so nine fit); the help line's id is swapped for ours on our row; the
  sysmsg lookup `0x1402CE7C0` is replaced (our ids `0x7F000001..4`). The choice is saved in the
  ini (`camera`), not the save.
- Untested in game: the row's layout against the help box, the text codes in sysmsg, the
  camera's turn direction and scale.

### 2026-10-16: Stage 67 (KH2 camera: right stick x the right way round)
- The right stick's x is negated for `kh2_camera` (the user: x was inverted).

### 2026-10-16: Stage 68 (KH2 camera at KH2's own distance)
- `camera_scale` defaults to 1.0 (KH2's distances and heights as they are), the user: a bit
  further back, as in KH2.

### 2026-10-16: Stage 69 (KH2 camera lowered)
- `camera_lower` (default 70): KH2's camera, look point and eye together, that much lower, so
  KH1's smaller Sora sits near the middle of the screen (the user). Same angle as KH2's.

### 2026-10-16: Stage 70 (KH1's camera in cutscenes)
- KH2's camera gives way to KH1's own follow / lock-on camera while an event runs (talks with
  text boxes included), an area script walks Sora, or KH1 holds him still
  (`takeover::kh1_scene`); KH1's carries on from where KH2's was. After a gap KH2's starts
  again from KH1's view (yaw and distance), not from where it left off.

### 2026-10-16: Stage 71 (the Jungle Slider is KH1's)
- KH1 movement state 30 (the Jungle Slider) hands Sora to KH1 for as long as it lasts (the
  user: he walked along the vines instead of sliding); KH2's camera gives way there too.
  `KH1_MODES` in `takeover.rs` lists such states.

### 2026-10-16: Stage 72 (no hanging on the sides of crates and tents)
- A fall the air mover stopped short counts as a landing only when KH1's collision has him on
  ground (`+0x2D0` bit 0 clear); a wall beside him stopped it too, and he hung on the sides of
  crates and the camp's tent, flickering between landing and falling (the user's video).
  Otherwise the fall keeps KH2's own height. On the ground he stays on such a landing while
  KH1's collision keeps him on ground there.

### 2026-10-16: Stage 73 (all of the Jungle Slider is KH1's)
- The slider also runs in KH1 movement state 31 (the user: its jump and attack didn't work, KH2
  had Sora there): every state from 30 up is KH1's (`kh1_mode`).

### 2026-10-16: Stage 74 (catching a ladder on the way down)
- The grab probe's air move presses 4 units along the stick instead of standing still: KH1's
  climb check (`0x1402C5D30`) needs the wall face the move touched (`+0x2DE`), which a
  standing-still move cleared, so a ladder jumped into was caught only at its foot (the user).
  Without a grab the position is put back.

### 2026-10-16: Stage 75 (a roll right after KH1's own move is KH2's)
- While KH1 has Sora back on the ground after one of its own air moves (`HANDOFF`, state 0/1),
  KH1 doesn't see Square, and a Square press ends the handoff at once with KH2 taking it as a
  fresh press (KH2's roll). Before, KH1 started its own roll there (slot 117) and KH2 took over
  mid-way: he slid backwards (the user, jumping off a pole and rolling).

### 2026-10-16: Stage 76 (KH2 camera on entering a room)
- A new KH2 camera starts at KH2's default distance, behind Sora unless KH1's camera was
  looking at him from a usual distance (then its yaw); and one is started when Sora is far from
  the look point (a new room, a warp). Before, it took KH1's left-over pair's distance, and
  KH2's closest distance puts the eye 150 below the look point: on the ground looking up (the
  user, every room entry).

### 2026-10-16: Stage 77 (no clashing with enemies' melee attacks)
- A KH2 swing meets only free-standing enemy shots (`+0x30` bit 0x400: thrown and fired
  things, Wakka's ball among them, whose log entries showed `+0x30 = 0x541`). Enemies' melee
  attack shapes no longer clash with it (the user, again: far too many clinks); the swing hits
  their bodies as in KH2.

### 2026-10-16: Stage 78 (KH2 camera parity)
- KH2's field of view: 69.9° vertical (1.5 rad across 4:3), set through KH1's FOV setter as 76.1
  "PS2 degrees" each frame KH2's camera runs (KH1's 53 is 48° vertical); KH1's default comes
  back when KH1's camera runs.
- Framed as in KH2: `camera_scale` 0.89 (KH1 / KH2 Sora's hips) and `camera_lower` 0.
- Sticks: the pad's raw ones with a 15% dead zone (KH1's processed ones drop the inner 47%);
  KH1's processed right stick when the raw one is idle (mouse).

### 2026-10-16: Stage 79 (New Ability box after the level-up's own)
- The "New Ability" box waits until KH1's prompt box has been free for 4 s: a level-up's own
  box comes a moment after the level changes and replaced ours (Aerial Dodge's was logged as
  shown, the user never saw it).

### 2026-10-16: Stage 80 (New Ability box beside the level-up's)
- KH1's prompt column holds up to 5 boxes per party member (`0xBA0` apart, count 16 bytes before
  the first). For 3 s after Sora learns an ability its box waits for KH1's own (the level-up's)
  and is added as the next box in Sora's column, with its own text buffers; with none by then it
  shows alone once the column is free (the user: show both at once).

### 2026-10-16: Stage 81 (Aerial Dodge after High Jump; grabs as KH1's jump tries them)
- Aerial Dodge is learned at level 13 only once Sora has High Jump (the user); without it, it is
  taken back out of his list, and its AP isn't in `bonus_ap`.
- Air grabs (ladders, vines, poles, ledges) are tried as KH1's jump step does: also near the top
  of the jump (rising under 2 a frame) and with the stick neutral (then no probe move: the
  frame's own air move left the contacts). The user: hit or miss from a jump.

### 2026-10-16: Stage 82 (bosses give KH2 moves)
- `growth::BOSS_GIFTS` (the user's list): Sabor Upper Slash, Clayton Aerial Sweep, Jafar Aerial
  Finish (KH1's Gravity Break), Pot Centipede Guard Break (Stun Impact), Dragon Maleficent
  Explosion (Hurricane Blast), Captain Hook Magnet Burst, Cerberus Aerial Dive. Learned once the
  world's story progress byte (save `+0x1504`, `0x142DEB264`, 12 worlds) reaches the value it has
  after the boss (thresholds as the Archipelago KH1 client reads them), unless Sora already has
  it; off until equipped, with a "New Ability" box. Upper Slash, Aerial Dive and Magnet Burst
  are no longer learned by level (`MenuAbility::level` = `BOSS`); their AP joins `bonus_ap` once
  the boss is beaten.

### 2026-10-16: Stage 83 (Aerial Sweep at level 28)
- `growth::LEVEL_GIFTS`: KH1's Aerial Sweep also at level 28 (the user; KH1's Dream Shield gives
  it at 84), unless Clayton already gave it. Guard Break stays at KH1's level 18 (the user).

### 2026-10-16: Stage 84 (Upper Slash works)
- Upper Slash's PTYA row (12, kind 0, motion 161) was never reachable (kind 0 rows aren't picked
  by `select`). As KH2's help has it: the action button (Square) in a ground combo's combo or
  finisher window starts it (forced row), when equipped. Retaliating Slash also takes the
  action button now (KH2's), besides attack. Menu help texts for both rewritten to say so.

### 2026-10-16: Stage 85 (Aerial Recovery has something to recover from)
- KH1's knock-back hits (kind 4) are KH2 blows now, as its launches (5) were; every 4 was a
  flinch, so Sora was hardly ever knocked into the air (the user: Aerial Recovery never came
  out). Aerial Recovery (jump) also works from an air flinch (hit while airborne).

### 2026-10-16: Stage 86 (KH2's Square moves)
- KH2's Square combo moves are kind-0 PTYA rows, never picked by `select`. The action button in
  a combo or finisher window now tries, in order, Finishing Leap (row 37, ability 95: the
  combo's end), Upper Slash (12, 18) on the ground and Horizontal Slash (38, 99) in the air,
  as KH2's help texts describe them (the user: Horizontal Slash didn't work). Menu help texts
  updated.

### 2026-10-16: Stage 87 (KH2 launches launch)
- Research: `docs/kh1-launch.md` (KH1's hit record, an enemy's launch reaction `0x1402B6390`,
  its arc fields). KH1 has no per-hit launch speed or angle.
- A KH2 hit whose ATKP reaction is a launch (category 4) forces KH1's launch (record kind 5, in
  `scale_record`) and, once KH1's launch reaction has run (hooked) and the enemy did launch
  (motion 0x52 / 0x5C, state 2), sets KH2's apex height and rise time (`+0x3BC` / `+0x3B8`, from
  the ATKP knockback values flown as KH2's blow flies) and horizontal speed (`+0x3C0`). Upper
  Slash goes up (the user). Enemies without a launch motion (big ones, most bosses) don't.

### 2026-10-16: Stage 88 (no attacks mid-launch; KH2 camera through summons)
- A forced combo row (Counterguard, Retaliating Slash, the Square moves) that can't be used is no
  attack: it fell back to the ordinary combo, so Sora attacked while knocked into the air (the
  user) and Square mid-combo could start a plain swing.
- KH1 holding Sora still counts as a scene for the camera only out of battle (the user: KH2's
  camera should stay through a summon's attack, Simba's charge).
- Launch logs: each KH2 launch asked for, and KH1 launch reactions that weren't ours.

### 2026-10-16: Stage 89 (a tapped Quick Run goes less far)
- Letting go of the action button before Quick Run's time is up stops it with a stronger brake,
  the earlier the stronger (0.7 per tick for a tap, KH2's stop brake when held to the end): at
  LV2 a tap went ~740 units against ~950 held, as KH2's end slide is most of it (the user).

### 2026-10-16: Stage 90 (KH2 camera through Simba's charge)
- Only the Jungle Slider's movement states (30, 31) give the camera to KH1; a summon's (38
  Simba's charge, 40 the call) keep KH2's (the user). KH1 still has Sora in all states 30+.

### 2026-10-16: Stage 91 (Quick Run grows with Sora; Aerial Recovery as the exe)
- The tap brake (stage 89) is gone (the user). Quick Run grows with Sora instead
  (`growth::QUICK_RUN_LEVELS`): LV1 at 12, LV2 at 20, LV3 at 32, MAX at 45. Its menu name is a
  buffer rewritten in place ("Quick Run LV2" ... "Quick Run MAX"), and each growth shows an
  "Ability Up" box.
- Aerial Recovery per `docs/aerial-recovery.md` (exe `0x1403D6030`): launches only (not air
  flinches), from the launch's 16th frame, jump: motion 40, hop 100, 10/frame along the stick
  (straight up with it neutral), invulnerable to frame 16, control back at frame 24, no Aerial
  Dodge after. Retaliating Slash: Square only, same frame.

### 2026-10-16: Stage 92 (Dodge Roll shows its level)
- KH1's Dodge Roll (22) gets a name buffer like Quick Run's ("Dodge Roll LV1" .. "MAX", its
  levels as before: 19 / 37 / 60), written into our ability table's entry, and an "Ability Up"
  box each time it grows (the user).

### 2026-10-16: Stage 94 (High Jump and Aerial Dodge grow with Sora)
- High Jump (KH1's 1, shared) and Aerial Dodge (our 67) level up like Quick Run and Dodge Roll
  (the user): High Jump LV2 / LV3 / MAX at 24 / 38 / 52, Aerial Dodge at 25 / 36 / 50
  (`HIGH_JUMP_LEVELS`, `AERIAL_DODGE_LEVELS`), LV1 when Sora gets them. Each gets a name buffer
  ("High Jump LV2", entry 1's handle like Dodge Roll's; Aerial Dodge's in the TEXT init like
  Quick Run's) and an "Ability Up" box when it grows; learning Aerial Dodge shows its level.

### 2026-10-16: Stage 95 (no leftover launches)
- The user's log: Upper Slash's launch reaches Shadows (up 265 in 26 frames), but launches
  asked on Cerberus (which KH1 never launches) stayed queued under KH1's reused hit record and
  one landed on a Shadow later. A KH2 launch now waits only 250 ms (`LAUNCH_FRESH_MS`) for
  KH1's launch reaction.

### 2026-10-16: Stage 96 (ground-to-air rows)
- The user: Aerial Sweep and Aerial Dive (ground-to-air) were hard to trigger. KH2's combo
  selector (`0x140401BB0`) measures facing from the direction to the primary target, not the
  player's facing; kind-1 rows have a facing window of 0.99996..1, so with Sora's own facing
  they (Aerial Sweep 33, Aerial Dive 48, the jump at an airborne foe 0, Slide Dash, Flash
  Step, the long approach 153, Aerial Spiral) almost never passed. `Controller::try_attack`
  now aims at the primary target for the selection. The height window's sign was checked too
  (`0x140401AD0`: target − player, y-down; the doc had it reversed, the code was right).
- Aerial Dive's help: from the ground, at a far airborne foe.

### 2026-10-16: Stage 97 (four shortcuts)
- The user: four shortcut slots like KH2, KH1's shortcut menu extended so it fits in.
  `win/shortcuts.rs`, `docs/kh1-shortcuts.md`. The fourth is L1 + the cancel button (Circle;
  Cross with the button swap), kept at save `+0x847` (spell + 1; old saves start empty), set
  in Customize (a fourth row, its label KH1's third with the cancel icon), filled with a newly
  learned spell when KH1's three are full. In battle the shortcut menu moves up a row, its
  panel is stretched by a row (cut at the middle, a one-texel band between), and the fourth
  row gets the cancel icon. `shortcuts = 3` in the ini turns it off.
- The text lookup (`config.rs`) is in even with `config_row = off`; an id past KH1's table reads
  as empty.
- The previous run's log is kept as `kh1-kh2.prev.log`.

### 2026-10-16: Stage 98 (the shortcut menu looks like KH1's)
- The user's screenshots: the battle panel cut in the middle looked wrong and the title
  ("SHORTCUT", the second panel: 8 px tall) moved into the rows (its quad is in GS
  coordinates, not the menu's, so the "lower rows" test always passed). The panel quad (header
  and three rows, 86 px, UV 1:1) is now drawn whole and its last 22 px drawn again a row lower;
  the title stays. In Customize the fourth label showed garbage: KH1's label 0xD0 is
  `0D 16 00 ...` (an x offset whose operand has a 0 byte), so it was cut short; the label is
  now read code by code (`0x1402CFEA0`'s operand sizes) and the confirm icon (`0F 41`) becomes
  cancel's (`0F 42`).

### 2026-10-16: Stage 99 (the gradient over four rows)
- The user: the shortcut panel's gradient should go all the way down. The panel is drawn as its
  header and four 22 px rows cut from KH1's three; row k shows texture row 2k/3, the two
  nearest rows blended (the second drawn over with vertex alpha), so the shading spreads over
  four rows and the shapes stay KH1's. The Customize label came out `0d 16 00 0f 42` (right).

### 2026-10-16: Stage 100 (lock-on camera steady)
- The user's video (KH2 camera, locked on in the Cave of Wonders): the camera jumped about.
  The "Sora is far from where it was looking" restart compared the look point with his feet;
  locked on, the look point is 30% of the way to the target, so a far target restarted the
  camera (from KH1's eye) again and again. It compares the eye now (1200). A lock-on target
  with no point this frame no longer hands the frame to KH1's lock-on camera: ours follows.
  Restarts and hand-overs are logged with their reason.

### 2026-10-16: Stage 101 (targets as KH2's target shapes)
- The user: Aerial Dive still barely came out. KH2's combo table measures a target by its
  target shape (`0x1403C8EE0`: the shape's point raised by its position height, and its own
  size), and Aerial Sweep / Aerial Dive / the jump at an airborne foe (flag 0xA: the window's
  low end shrinks by the target's height) want that point at least 110 + its size above Sora.
  We gave targets their feet and whole height (a flyer needed its feet ~200 up). Every target
  is now KH1's lock point 0 (its centre, the lock radius as its size), the locked one too (it
  was the sphere's bottom and twice the radius); a target without lock points keeps its feet
  and height. Magnet Burst's pull still aims at the body's middle (`Body::tall`).

### 2026-10-16: Stage 102 (the magic carpet)
- The user's video: the magic carpet out of the Cave of Wonders (after Genie Jafar) didn't
  move. The log: KH1's control step for Sora came and went every few frames ("KH1 holds Sora
  still" / "lets Sora act again" over and over), and KH2 took him on the frames between, so
  KH1's own physics (which the ride needs) didn't run. KH1's control coming and going 6 times
  in 3 s is now taken for a ride: KH1 has him while it lasts and 3 s after. The control step
  is read once per frame at the start of `takeover` (`FROZEN_NOW`), before any early return.
  The hold / act lines now carry his state, slot, flags, motion and the object he stands on.

### 2026-10-16: Stage 103 (Aerial Finish holds)
- The user: enemies were pushed out of the air finisher. Aerial Finish's spinning hits
  (ATKP 915 / 916 / 917: reaction 9, up 10 / 5 / 12, sideways -1 / -1 / -10) are blows that
  hold the foe up beside Sora; landed with KH1's air finisher row they sent it ~250 away on the
  first hit (the log: 915 at z -1836, 916 at -2089) and the rest missed. A blow takes KH1's
  finisher row (and a KH2 launch) only when it throws: sideways >= 10 or up >= 30 (or the
  default blow); the others land with the plain row. The last hit (927: 3 / 45) still throws.

### 2026-10-16: Stage 104 (invulnerable frames)
- The user: Aerial Recovery, Aerial Dodge and Quick Run should have invulnerable frames.
  Aerial Recovery already had KH2's (to frame 16). Aerial Dodge (none in KH2) gets 10 / 12 /
  14 / 16 by level (`AIR_DODGE_STAR` + 2 per level). Quick Run keeps KH2's fmab star times
  (0 / 8 / 12 / 16 by level) with at least 6 (`QUICK_RUN_STAR_MIN`, for LV1).

### 2026-10-16: Stage 105 (Riku's Aerial Dodge, later growth, holding hits stay put)
- The user: Aerial Dodge for beating Riku the first time (Hollow Bastion progress 0x32, its
  reward's threshold in gaithern's KH-1FM-AP-LUA, MIT; 0x28 during the fight), no longer a
  level or High Jump's (the High Jump gate and its take-back are gone; the user's Archipelago
  save has no High Jump). Its New Ability box shows its level. High Jump grows at 30 / 44 / 58,
  Aerial Dodge at 40 / 52 / 64.
- Still sliding out of Aerial Finish: KH1's flinch on an airborne foe (kind 3, `0x1402B69E0`)
  knocks it along the record's direction each spin. A holding blow's record direction is now
  zeroed (in `scale_record` and again after the contact), so it flinches in place.

### 2026-10-16: Stage 106 (no Sonic Blade in the air)
- The user: Sora could use Sonic Blade mid-jump. KH1's command check (`0x1402912F0`) allows
  only magic (kind 2), kind 99 and kind 0x47 in the air, judging the air by his flags
  (`0x1402A0D30`: `+0` bit 4, `+0x88` bit 0x20, `+0x444` bit 1), which stay the ground's while
  KH2 has him. `combat::command` now refuses any other command while KH2 has him in the air,
  with KH1's "can't" sound (`0x140177F60 (4)`). Magic stays allowed (jump casts).

### 2026-10-16: Stage 107 (Aerial Dodge taken back until Riku)
- The user's save still had Aerial Dodge (from its old level). It is taken out of Sora's list
  while Riku (Hollow Bastion, first time) isn't beaten, and given again (with its box) once he
  is.

### 2026-10-16: Stage 108 (KH2's sounds for every ability move)
- The user: the exact KH2 sounds for the KH2 moves, as Explosion's and Magnet Burst's (no
  visuals). `kh2sound::DEFAULT_MOTIONS` is now every ability motion of Sora's PTYA (161..172,
  191..197: Upper Slash, Slapshot, Dodge Slash, Slide Dash, Guard Break, Explosion, Finishing
  Leap, Flash Step, Vicinity Break, Counterguard, Retaliating Slash, Aerial Sweep, Aerial
  Spiral, Horizontal Slash, Aerial Finish, Aerial Dive, Magnet Burst); the plain combo keeps
  KH1's. `kh2_effects` keeps its own two (`kh2fx::DEFAULT_MOTIONS`). Any `kh2_sounds` word
  but `all` / `off` (the old `magnet_explosion` too) means the new default.

### 2026-10-16: Stage 109 (no Sonic Blade in the air, again)
- The user: Sonic Blade still came out mid-jump; the log had it as a ground command (command
  0x4, not "in the air"). `kh2_airborne` read the driver with `try_lock`, and KH1's command
  step can run while it is held (a failed lock read as "on the ground"): the takeover now keeps
  `KH2_AIR` each frame. The refusal also runs while KH1 has him in its fall state 2 (after one
  of our air casts, whose state we set without KH1's airborne flags), before the hand-off
  test, and every command logs whether KH2 had him in the air.

### 2026-10-16: Stage 110 (the ride test only while KH2 has him)
- The user: Sora stuck in KH1's animations. Stage 102's ride test counted KH1's control coming
  and going on every frame, KH1's own included (a room entry's script holds, then KH1's own
  frames, which flicker too), and each count kept the ride going: KH1 had him for good. It now
  counts only on frames KH2 has him, needs 10 changes in 3 s, and gives KH1 him for 5 s
  without extending.

### 2026-10-16: Stage 111 (the magic carpet, for certain)
- The user can't easily get back to the carpet and it softlocks if broken. `0x1402957F0` (the
  player actor's setup) puts Sora in movement state 29 (0x1D) in Agrabah (world 8) room 0x14
  while its event index is under 0xD: the carpet out of the Cave of Wonders. KH1's own modes
  were 30 and up; now 29, 24 and 25 too (24 is Atlantica's water, set when `0x14298D3A0`, world
  9 or End of the World room 0x15, `0x140288920`; 24 / 25 are the states where jump means up,
  `0x140291660`'s mask 0x203000100). KH1's camera for 24 / 25 / 29 / 30 / 31. (Stage 110's
  flicker test stays as a fallback.)

### 2026-10-16: Stage 112 (KH2-named KH1 abilities earlier)
- The user: Vicinity Break at 69 (Dream Shield) is absurd, Slide Dash at about 30. `LEVEL_GIFTS`
  now also gives Counterguard (Counterattack) at 22, Slide Dash (Sliding Dash) at 30, Vicinity
  Break (Ripple Drive) at 36, Flash Step (Vortex) at 42 and Finishing Leap (Blitz) at 48, with
  Aerial Sweep at 28 as before (off, with a New Ability box, if Sora doesn't have it yet).

### 2026-10-16: Stage 113 (Aerial Finish holds its foe)
- The user: foes still slipped out of Aerial Finish. The log: whole spins land sometimes, but
  often the foe is gone after 915 / 916 (falling while Sora spins up). KH2's reaction-9 blows
  lift and pull the foe toward Sora; KH1's flinch lets it fall. `Strikes::hold`: a foe a
  holding blow hit is drawn, for 24 frames after each such hit, toward a point no more than 90
  from Sora at his height + 30, through KH1's per-frame push (`+0x120`, at most 30 a frame).

### 2026-10-16: Stage 114 (Gummi routes row, chests in battle)
- The user: a Config option to skip the Gummi routes or fly them; chests in battle.
- `config.rs` now has two rows of ours (8 Camera Style, 9 Gummi Routes: Vanilla / Skip), ten
  rows 22 apart while the list runs, help ids swapped in at the table's slots 8 / 9;
  `settings::save(key, value)` keeps either in the ini (`gummi = vanilla / skip`).
- `gummi.rs` ports KH-1FM-AP-LUA's Instant Gummi (denhonator, Gicu; MIT; credited in NOTICE /
  CREDITS) to the pad read each frame: the chosen destination becomes the flight's current
  one (Epic addresses: selection 0x507D7C, destination 0x508280, 0x507C90, 0x268A26C, warps
  0x50F9D0).
- `chests.rs`: the battle check before the chest prompt (`0x1402B3904`: `test [0x142867CD8],
  1; jz`) becomes `jnc`, as the AP interaction script (KSX, Gicu) does. `chests_in_battle`.

### 2026-10-16: Stage 115 (big targets by their parts)
- The user: Sora couldn't hit the glowing blobs on Oogie's Manor. Likely parts of the manor
  actor, whose root is far from Sora: `combat::bodies` dropped any actor whose root was more
  than 1500 away. It now keeps an actor when any of its parts is within 1500 (KH1's own hit
  pass tests every part), and logs big targets (more than 8 parts: parts hittable, root
  distance) for the next look.

### 2026-10-16: Stage 116 (big targets aren't pulled)
- The user: Aerial Finish (and likely Magnet) on Oogie's Manor's blobs pulled the whole house
  into the floor: our pulls push the whole actor (`+0x120`). A body is `fixed` (more than 8
  parts, root beyond 1500, taller than 600 or radius over 300) and the Magnet pull and the
  Aerial Finish hold skip it. Stage 115 and 116 shipped together.

### 2026-10-16: Stage 117 (dev tool: insta-kill)
- The user asked for a dev tool to toggle insta-killing enemies. New `win/devtools.rs`:
  `insta_kill = on/off` in the ini (default off); F8 (monitor thread) or L1 + R1 + Select (pad
  hook) toggles it and saves it with `settings::save`; menu sound 2 (on) / 3 (off), played on
  the game's thread from the pad hook. With it on, `probe::hit` (every hit record from an
  attack Sora owns: KH2 swings after `scale_record`, KH1 swings, magic) gets damage = the
  foe's HP (stats `+0x3C`), at least 9999.

### 2026-10-16: Stage 118 (Neverland flight)
- The user: Sora couldn't fly in Neverland. KH1's flight is movement state 8, not 25 (25 is
  Mermaid Kick's dash, from swimming state 24 with KH1 ability 2). State table at
  `0x142DB4240` (0x40 per state), filled by `0x1402AF9F0(state, flags, enter, exit, ...,
  move +0x20, next +0x28, ..., +0x38)`; the setter is `0x1402AF7D0(actor, state)`. The fall
  state (2) starts flight when the world flags `0x142867CD8` have bit 0x20 (Neverland) and
  `+0x3D4` (height above the floor) is over 100 (`0x1402AFD30`: motion 0x75, state 8,
  `+0x3C4 *= 0.3`), or by a per-room height table (`0x1402AFC10`, `0x140529970`: world 0xD
  rooms 8 / 9, world 0x10 rooms 0x1A / 0x21 / 0x22). KH2 has the jumps, so KH1's fall state
  never ran: `takeover` now does that check while KH2's jump comes down and calls KH1's flight
  start; state 8 is KH1's own mode (and camera) until he lands.

### 2026-10-16: Stage 119 (KH2 camera in flight)
- The user: flying snapped to KH1's camera with KH2's chosen. Flight (state 8) runs in KH1's
  mode-1 follow camera, which is ours, so state 8 left `kh1_camera_mode` (still KH1's own
  movement mode for `takeover`).

### 2026-10-16: Stage 120 (KH2 aerial attacks while flying)
- The user asked for KH2's aerial attacks in flight. `combat::exec`: the Attack command while
  Sora's KH1 state is 8 sets `takeover::FLY_ATTACK` instead of KH1's flying attack. `takeover`
  then puts him in state 0 (flight's exit step runs), KH2 starts fresh in the air
  (`d.grounded = false`) with that press (`FLY_PRESS`, after the fresh start's dropped
  presses; the "KH1's own action" slot check skipped that frame), and `FLEW` lets the flight
  start again once the combo is over (KH2 state Air / Jump, coming down) at any height above
  20, not only above KH1's 100; cleared on landing.

### 2026-10-16: Stage 121 (no sinking in a flight combo)
- The user: Sora should stay in flight through the combo. While `FLEW` and KH2 is attacking
  (Attack / Magic / Aerial Dodge, in the air), KH2's downward displacement is dropped (rises
  kept); the flight takes him again when the combo ends.

### 2026-10-16: Stage 122 (ladders caught on the way up)
- The user: jumping into a ladder didn't put him on it at once. KH1's jump step `0x1402ADA90`
  tries its grabs every frame, rising too; it skips them only with flag 0x10000 for 15 frames
  (`0x1404308B0`) after jumping off something, or while `+0x3B0 <= +0x3B8 + 6` when `+0x3B8`
  is set. Our "rising under 2 a frame" limit (stage 81) is gone.

### 2026-10-16: Stage 123 (flight combos: sparkle, homing, no landing)
- The user: keep the flight sparkle through flying attacks, home in on the enemy (down too),
  and don't end the flight when a move touches the ground unless Square (descend) is held.
- Sparkle: Sora's step `0x140295E50` keeps its effect at `+0xB0` (`0x1402ABA90(0x142D67400,
  0x14, sora, 0x8003)`) while state 8 and ends it (`byte = 1` on its `+0x10`) otherwise.
  `takeover::sparkle_stub` replaces `cmp [rdi+0x70],8; jnz; mov eax,[rdi]` at `0x14029602E`
  with a call to a stub that also answers "flying" while its flag byte is set (a flight
  combo: `FLEW` and airborne). (The 0x42D4 flight sound and the low-flight dust `0x1402BFEB0`
  are flight's move step's own.)
- Homing: KH2's approach is level; in a flight combo's attack the feet go toward the aimed
  target's point less 60 (lock, else the nearest within 600) at 0.15 of the gap a frame, at
  most 14.
- No landing: in a flight combo the fall stops 15 above the floor unless Square is held; the
  flight takes him again from 5 up.

### 2026-10-16: Stage 124 (no Superglide; Glide grows) and 125 (Aerial Dodge from Genie Jafar)
- The user: remove Superglide; Glide levels with Sora, LV2 at 40 up to LV3; Chernabog gives
  Glide MAX instead of Superglide. `growth::GLIDE_LEVELS = [40, 52]`, `glide_level(level,
  progress)` (4 once `CHERNABOG`, End of the World index 11 at 0x33, the AP client's
  Superglide check); `win/menu.rs` turns Superglide in the shared list into Glide (or drops it
  beside one), names Glide with its level and Superglide's table entry "Glide MAX" (KH1's own
  Chernabog reward box), and an "Ability Up" box for LV2 / LV3.
- Also (the user): Aerial Dodge is Genie Jafar's gift now (Agrabah index 4 at 0x53: 0x46 during
  Jafar, 0x53 after Genie Jafar's cutscene, KH-1FM-AP-LUA's warp fixes), not Riku's, and grows
  from 30: `AERIAL_DODGE_LEVELS = [30, 42, 54]`.

### 2026-10-16: Stage 126 (every enemy counts for the combo table)
- The user: Magnet Burst hardly came out. KH2's Magnet Burst is an air finisher: row 49 (kind
  2: any target within 650 whose direction is more than ~22 degrees off the aim, facing window
  -1..0.924) and row 50 (kind 1: the primary at 300..650). With `auto_aim` off the hook cut
  `Env::targets` down to the lock, so kind-2 rows (Magnet Burst's, Explosion's 26, Vicinity
  Break's 51, 34 / 59 / 43) only ever saw the primary, dead ahead. Now all targets go in and
  `Character::set_auto_target(false)` keeps the aim on the lock only (no lock: no primary).
  Flight-combo vertical homing follows the lock (or the nearest with `auto_aim`). Its help
  text says "Air combo finisher".

### 2026-10-16: Stage 127 (crash after Riku 1: GPU device lost; notes and a safer ring)
- The user: the game crashed after beating Riku 1. The log: `CRASH 0xc0000005 at
  exe+10ab69 (reading 0x0)` in `0x14010A990`, KH1's frame-fence wait reading a finished
  list's GPU timestamps: `ID3D12Resource::Map` (vtable +0x40) on the list's readback buffer
  gave null, which it does once the device is removed (hung / reset). Cause not in the log.
- The crash note now adds KH1's device's `GetDeviceRemovedReason` (vtable 37), our KH2 effect
  draw count and how long ago the last was (`fx12::crash_note`).
- `fx12` vertex ring: a write point going round the whole 24 MB ring (256-byte aligned)
  instead of one 6 MB slot per draw call, which came round again within KH1's frames in
  flight when a frame had several effect draws.

### 2026-10-16: Stage 128 (no KH1 combo during KH1's own moves)
- The user saw KH1's Ripple Drive graphic twice as Riku 1 fell (right before the crash). The
  log's last hits were KH1's own attack objects (`probe::hit` with no KH2 power): Attack
  pressed while KH1 still had Sora after a spell (`hand_to_kh1`, "command 0x2") ran KH1's own
  combo (its finisher: Ripple Drive, equipped). `combat::exec` now takes that press for KH2
  (`takeover::ATTACK_AFTER`): the move ends as soon as he stands free (state 0 / 1, 10
  frames in, as KH1 allowed the cancel) and KH2's first frame gets the press
  (`FLY_PRESS_GROUND`, past the fresh start's dropped presses). Cleared on each new hand-off.

### 2026-10-16: Stage 129 (KH1 moves end on the stick; no spare early gifts)
- The user: after a Hollow Bastion crystal (command 0x5), moving within a second kept KH1's
  animations: the hand-off ended only once KH1's slot was a plain idle / walk / run (0..7), and
  KH1 walked him in other slots (75 / 77). Now it also ends when KH1's own "can act" test
  (`0x1402956E0`: no action flag 4, no reaction `+0x88 & 0x20`, `+0x444 & 1` clear) passes
  with the left stick (pad bytes 2 / 3) past 0.3, state 0 / 1, 10 frames in.
- The user: a second Slide Dash from Maleficent. `growth::early_kh1()` (the KH1 ids of
  `LEVEL_GIFTS` and `BOSS_GIFTS`) keeps one copy each in Sora's list (`keep_one`, the
  equipped one); KH1's own later copy is taken out (logged, and in `kh1-kh2-removed.txt`).

### 2026-10-16: Stage 130 (no "learned" line for spares; fewer clinks; nothing big or fixed moved)
- The user: remove the learned box for duplicates. KH1's event command `0x1401B6990(event)`
  adds a reward's line to a message window (reward id at `+0x198 + sp * 4`, `sp` at `+0x194`;
  record `0x14028D810(id)`: u16, kind = low 4 bits (1 ability), member bits 4..7, ability the
  high byte; it pops 2, returns 2); abilities are added by `0x1401AF120` through
  `0x140282CA0(id, member)` (appends `id | 0x80`). Hooked (`menu::reward_line`): Sora's ability
  that `Kh1Sora::spare_gift` (the mod's level / boss gift is due and he has it) is popped with
  no line. Untested: a window whose only line it was may show empty.
- The user: Darkballs and Dragon Maleficent clinked way too much. Their guards (priority 100,
  objects 330 / 332, flags with bit 20 "spent", which only skips KH1's wall test in
  `0x1402B1670`) repelled swings whose wide KH2 shapes touched a guard beside the part they hit.
  A guard repels now only when it is met earlier along the frame's sweep than any body part.
- The user: Magnet Burst and Aerial Finish shouldn't affect Dragon Maleficent, big enemies, or
  anything immovable. `fixed` also covers actors that aren't KH1 kind 3 (attackable objects);
  a fixed body gets no KH2 launch (`PENDING`, which forced KH1's launch reaction), no Aerial
  Finish hold and no Magnet catch: KH1's own reaction stays.

### 2026-10-16: Stage 131 (KH1's clash table for guards; big stays big)
- The user: Dragon Maleficent was still pulled into a wall (Aerial Finish, Magnet Burst): her
  size was judged each frame from her pose (615 tall standing, under 600 head down). A target
  found big (more than 8 parts, taller than 600, radius over 300) stays big (by actor and
  object id).
- The user: high aerial combos on her clinked. Her big guard (radius 150, flags `000d6a09`) is
  clash type 6, which meets nothing in KH1's clash table (`0x1405299B8`: `1F 09 09 3F 29 18 00
  00`). Guards now repel only when the table lets their type meet Sora's swing type (his KH1
  row data's `+0x38 & 7`); logged ("guards near").

### 2026-10-16: Stage 132 (guards meet a thin blade)
- The user's Dragon Maleficent video: half the air combos' first swings were repelled by her
  guard (radius 180, flags `00116a09`, clash type 0, motion-bound), the other swings hit her
  head / neck. KH1's swing is one sphere of its data's `+0x5C` radius on the blade; KH2's hit
  shapes are wide and clipped the guard's edge on the way to the head. Guards are now tested
  against the sweep with each shape's radius cut to that KH1 radius (8..60).

### 2026-10-16: Stage 133 (no KH1 glide after an air spell; fewer guard repels)
- The user's video: after a spell in the air the hand-off ended with KH1 still in its air
  state (1, then 2); its fall state's step (`0x1402ADBD0` -> `0x1402B0530` -> `0x1402B0BA0`,
  with Glide's flag `0x142D5F610 & 0x80` and the jump button held for KH2's glide) put him in
  KH1's glide (state 0x15, motion 0x71). A hand-off now ends by setting KH1's state to 0
  (`0x1402AF7D0`, its exit steps run) and starts KH2 in the air when he is (`AIR_START`); KH1's
  glide state under KH2 is turned back to 0 at once (logged).
- Clinks: Sora's swing data (bone 0x30) has no radius / priority of its own (the Keyblade's
  are added by `0x1402B10E0`), so "blade radius 8" was only the clamp. KH1 itself clashes a
  type-0 priority-100 guard with any swing entering it. Now, with a part touched in the same
  frame, a guard repels only when that part's centre is inside the guard (the blade must pass
  through it); repel notes log the guard's motion tie, the owner's motion and the nearest part.

### 2026-10-16: Stage 134 (Heartless form is KH1's; damage fractions carried; damage_scale)
- The user: Heartless Sora (Hollow Bastion) had no jump animation, and should have no KH2 at
  all: KH2 drove him while his body was another model, left in KH1's idle. `pose::sora_body_seen`
  (Sora's own 293-bone model posed within 500 ms, from the hands' timestamp) gates `takeover`:
  otherwise KH1 has him ("Sora is in another body").
- The user: more damage, Explosion should do a lot more. Its burst is ~20 hits at power 25
  plus a 150 blast; each hit is KH1's damage x power / 100, which was floored (6 x 25% = 1.5 ->
  1, a third of the burst). The fraction is now carried per foe (`CARRY`, 1/10000ths, the foe
  from the part `land` hits: `TARGET`), and the ini's `damage_scale` (default 1.0, 0.1..10)
  multiplies Sora's KH2 hits.

### 2026-10-16: Stage 135 (Magic-based KH2 hits grow with MP)
- The user: Explosion and Magnet Burst should do more; they scale with magic. KH2's ATKP stat
  source byte says so: Explosion 911 / 912, Magnet Burst 1900 / 1901 / 1903 are 1 (Magic).
  KH1 has no Magic stat: such hits (`MAGIC` while landing) get x (1 + 0.1 x Sora's max MP, save
  member `+8`) x the ini's `magic_scale` (default 1.0) on top of `damage_scale`.
- The user hadn't seen Vicinity Break: it did come out twice in the log (motion 170, attack
  1869, both hits landed). Its row (51) wants a foe behind Sora within 300 (facing window
  -1..-0.17), which before stage 126 never had any foe but the lock. Its KH2 visual stays off
  (the user asked for only Explosion's and Magnet Burst's).

### 2026-10-16: Stage 136 (Vicinity Break's KH2 visuals)
- The user asked for them: `kh2fx::DEFAULT_MOTIONS` is Explosion (166), Vicinity Break (170)
  and Magnet Burst (197); the ini's `magnet_explosion` value now means those three.

### 2026-10-16: Stage 137 (Flash Step for Sabor)
- The user: Flash Step (KH1's Vortex) unlocks after Sabor. Out of `LEVEL_GIFTS` (it was 42),
  into `BOSS_GIFTS` (Deep Jungle index 1 at 0x42, beside Upper Slash). Still one of
  `early_kh1()`: KH1's own later Vortex is a spare.

### 2026-10-16: Stage 138 (Glide on Square)
- The user: Glide on Square, as KH1's Superglide. The controller's `GlideButton::Action` (KH2's
  own option): Square in the air glides; ini `glide = square` (default) or `jump`. Out of a
  flight (`FLEW`) it stays on jump: Square is flight's descend there.

### 2026-10-17: Stage 139 (Gummi skip and insta-kill removed)
- The user, to keep the mod modular: the Gummi route skip is gone (`win/gummi.rs`, the
  `gummi` ini key, the Config menu's Gummi Routes row: back to one row of ours, Camera Style,
  rows 25 apart) and so is the insta-kill dev tool (`win/devtools.rs`, `insta_kill`, F8 /
  L1 + R1 + Select). NOTICE / CREDITS no longer list Instant Gummi.

### 2026-10-17: Stage 140 (release package)
- The user is releasing the mod. Fresh installs default to `abilities = kh1` (was `test`);
  `abilities = remove` (= test mode) takes the KH2 abilities back out of a save before
  uninstalling. The DLL checks the game build first (`supported_exe`: the first bytes of
  KH1's physics step `0x298230`, control step `0x2A4090` and reward line `0x1B6990`, the
  Epic Games Store release) and stays off otherwise (Steam's addresses differ). `fx_test`
  left out of the default ini. The mod.yml describes the mod for players; the release zip is
  mod.yml + kh1-kh2.dll + README.txt (install steps) + licences, NOTICE, CREDITS.


### 2026-10-17: Stage 141 (no chests in battle)
- The user: remove the chests-in-battle patch. `win/chests.rs` and `chests_in_battle` are gone;
  chests stay closed in battle, as in KH1. NOTICE no longer carries KH-1FM-AP-LUA's licence
  (no code from it is left); CREDITS keeps a facts-only line for its story-progress values.
  The prompt-box layout (`menu::prompt`) is checked against KH1's own box updater
  `0x140270DA0`: state at `+0`, timer `+0xC`, line count `+0x18`, lines `+0x20` / `+0x28`,
  title `+0x30`, fade `+0xB80`, colours `+0xB88` / `+0xB90`, 0xBA0 a box, counts per member
  at `0x14283BD80`, members `0x3A20` apart.

### 2026-10-17: Stage 142 (Steam release)
- The user now has the Steam release (`D:\Steam\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5
  ReMIX-`); players reported the mod doing nothing there (it switched itself off). Same game,
  linked the same day (Epic stamp 1721648200, Steam 1721645006), but each build links its store's
  SDK, so code and globals shift by region (e.g. battle flags `0x2867CD8` -> `0x2867364`, save
  `0x2DE9D60` -> `0x2DE9360`).
- `win/addr.rs`: every game address is an `Rva` (the Epic RVA); `base + rva` gives the running
  build's address (`impl Add<Rva> for usize`), from `steam_map.in` (Epic -> Steam, 174 pairs) on
  Steam. `addr::detect` picks the build by the PE link stamp and checks three prologues; any
  other build stays off as before. `tests/steam_map.rs`: every `Rva(0x..)` in `src/win` has a
  Steam address.
- How the table was made (locally, from the user's two executables; nothing of them is in the
  repo): functions paired through `.pdata` and a diff of their instruction streams with
  relative targets set aside (14,827 of 15,824 identical); globals from the RIP-relative
  operands of paired functions. Each code address was checked over 64 bytes, instruction by
  instruction, with each reference mapping consistently; menu tables with no direct reference
  (`0x5470E0`, `0x547130`, `0x547140`) by their contents (-0xCF0, as `0x5470C0`). Patched bytes
  (shortcut rows, Customize, sparkle test, combo table) are identical in Steam's build.
- Steam's English files are in `Image\dt` (the KH1 / KH2 loaders already try en, dt, jp).
- README / mod.yml: Epic or Steam; README.txt kept in `crates/kh1-hook/openkh/`.

### 2026-10-17: Stage 143 (Steam: first test)
- The user's first Steam run (new game): the build was recognised and KH2's control ran, but
  two hooks were refused: menu text lookup (`0x2CE7C0`) and shortcut get (`0x282F20`, and so
  set `0x283640`) begin `mov rax, [rip + disp]`, whose offset differs per build. Their
  expected bytes are now made for the running build (`addr::mov_rax_rip`: text table
  `0x2E1B650`, save pointer `0x28695A0`); the trampolines are never called (both replaced
  outright). Every other hook's prologue checked against both executables: identical.
- Dive to the Heart's "Press to attack" window stayed up although KH2's swings add to the
  attack tally (`0x14028A4D0(0x13)`, identical in Steam's build; read by event opcode 474,
  `0x1401ADFA0`). Diagnostics: event opcodes per second when they change (dispatcher
  `0x1401B13F0`, table `0x1404FB550`, 764 opcodes; `probe::event_trace`) and camera jerks
  (`camera::jerk_note`: the user found the camera jittery there).

### 2026-10-17: Stage 144-145 (the attack tutorial; camera timing)
- The trace (`probe::event_trace`): at Dive to the Heart's "Press to attack" window the script
  polls opcode 322 (`0x1401AB6E0`: pushes the script actor's `+0x164`) every frame, with Sora as
  the actor. `+0x164` is the motion request applied (`0x14029DAB0`: low 16 bits of the first
  queued request): 0 idle, 1 walk, 2 run under KH2; KH1's swing starts the row's motion
  (`0x1402A17F0` -> `0x14029D7F0(actor, motion | 0x1000000)`, 200 for the first ground row),
  which the window waits for. Stage 50's tally (`0x14028A4D0(0x13)`) was not what it reads.
- Fix: `combat::exec` notes the row's motion when KH2 takes an Attack press
  (`attack_shown`); for 0.6 s after, opcode 322 run on Sora has its pushed value replaced
  with it (`probe::event_op`; the slot at `+0x198 + count * 4`, count `+0x194`, 0 for the first slot: the s145 build skipped 0 and never fired). Sora's own
  `+0x164` is untouched (KH1's attack objects tied to a motion compare it).
- Camera (`camera::jerk_note` summaries): the eye and look point move smoothly (≤ ~1 unit of
  change per frame walking) at 60 updates/s, but frames arrive 2..42 ms apart even standing
  still: uneven frame pacing, not the camera.
- The user's "slight slide" in the scene after "move and run": the log shows KH1's own scene
  motions there (slots 206 / 207, `follow` leaves KH1's animation): KH1's own walk.

### 2026-10-17: Stage 147 (the user's bug list)
- Attack on the first free frame after a KH1 move was KH1's attack, and KH1's combo ran on
  from there (out of order, glitched): `kh1_move` needed `active()` (KH2 drew a frame in the last
  150 ms), which lapses during any KH1 move longer than that. Now `HANDOFF_SEEN_MS` (the
  hand-off branch seen in `takeover`) keeps it; and `combat::exec` never runs KH1's attack row
  for Sora while `takeover::kh1_borrows` (his own body, no event / area script / ride / KH1
  movement mode / flight): it hands to KH1 with `ATTACK_AFTER`, KH2's first press when KH1 lets go.
- Confirm button on Circle (Options): KH1's pad step `0x140288F30` maps Cross to confirm
  (`0x210001`) and Circle to jump / cancel (`6`), swapped when `0x1402F4690` (a settings field;
  `+0x1A` Epic, `+0x1E` Steam) says so. We hid Circle from KH1 and read Circle as KH2's jump,
  so with the swap KH1 saw its jump and took Sora. `takeover::jump_button()` follows the option
  (pad mask and KH2's jump).
- L1 + jump cancelled KH2 moves when no shortcut was set: KH1 opens its shortcut menu on L1
  only then (`HAS_SHORTCUTS_RVA`), else the face buttons are its plain ones. The pad mask and
  KH2's `shortcut` now use `shortcuts::menu_on_l1`.
- Commands (magic, items, specials) cancelled any KH2 move: `combat::command` now runs one only
  when `can_cast` (spells: free, landing, an attack's ComboMagic window) or `can_take_command`
  (others); otherwise it waits (`PENDING_COMMAND`, KH1's 16-byte record) and
  `resume_command` (before KH1's control step) puts it back once allowed, within 1.5 s.
- Pause menu: KH1 poses a second copy of Sora's model there; any 293-bone instance was taken
  for Sora (one frame of KH2's pose, and KH2's walk at the menu Sora's zero speed: slow). A new
  instance is Sora only within 150 of his actor (`pose::at_sora`); the other is left to KH1 and
  noted (`menu_sora_within`). KH2 effects aren't drawn while it is (the menu's world pass drew a
  copy), and KH2's camera carries on across a pause (spamming pause restarted it unlocked).
- Keyblade trail: KH1 drops a trail asked for 30+ frames before the blow (the modifiers' long
  wind-ups: "not drawn" for motions 169 / 192 in the logs); asked again as the hit window opens.
- Aerial Finish (motion 194, combo row 29, ability 100) joins KH2's effects moves.
- KH2 effects step on KH1's frames too (`kh2fx::NO_MOTION`: the cut-short move's end); a spell
  mid-move froze them.

### 2026-10-18: Stage 148 (a Steam tester's log; the user's videos)
- A tester's log (Steam, s146 release): every pad record logged empty ("0000", sticks 80) the
  whole session, two record addresses alternating, while KH1 walked, jumped (`set motion slot
  9`, "KH1's own action, slot 12, KH1 state 1"), glided and ran its own air combo; only Attack
  and magic (KH1's command menu) reached KH2. Keyboard play: KH1 merges its keys in after the
  pad records (`0x140288F30`), which is all KH2 read. Their ini also had `commands = off` (KH1
  runs its commands itself, any time), `kh2_voice = on` (KH2's lines: the "wrong quotes"),
  `kh2_camera = off` (KH1's own camera, its own lock-on behaviour).
- `takeover::buttons()`: the pads' records or KH1's merged state (`0x14298D4A0 + 4` held: `6`
  as the jump button, `0x100000` as Square); `move_tilt()`: the pad's stick or KH1's processed
  one (`0x14298D4E0`). `probe::merge_pads`: every record seen in the last 100 ms merged.
- "KH2 has him" is now by game frame (`FRAME_KIND`, set each physics frame of Sora: KH2 / KH1's
  hand-off move / KH1), not 150 ms of time: a pause no longer drops KH2 (the battle pause's
  aerial combo went to KH1's table).
- KH1's control step (`takeover::control`) runs without its jump / cancel (`6`) and Square
  action (`0x100000`) bits while Sora is KH2's or KH1 has him outside one of its moves
  (`kh1_borrows`, no hand-off, no L1 shortcut menu): KH1's merged state, so the keyboard, every
  pad and the confirm swap alike; put back after (menus keep them). KH1 jumped on the first
  frame after a pause or an examine.
- Back from a command's hand-off (`FROM_COMMAND`): KH2's battle stance (a standing spell left
  him in the field idle).
- KH2's camera started while locked on faces the target at once (it came back unlocked after a
  pause, the battle pause, a limit, the save menu or examining).

### 2026-10-18: Stage 149 (command submenus, the spell queue, room entry)
- The command menu's current level: `0x142D54450` points at `&0x142D543F0` at the top level
  (there KH1's menu step `0x140291660` turns cancel, pressed bit `4`, into the jump flag
  `0x1000000` on the command record) and deeper in a submenu (magic, items, summons), where
  cancel backs out. `shortcuts::submenu_open()`: the pad mask, KH2's jump and the control-step
  strip leave Circle to KH1 then (the user couldn't leave those lists during a KH2 move).
- The command queue waits up to 5 s (a combo with its finisher; the user: a spell chosen
  mid-combo never came), keeps the menu step's own flags when it puts the choice back, and
  logs every wait / resume / drop on its own count (`cmd_log`).
- `pose::at_sora` compares a new model with Sora's actor from KH1's party slot
  (`takeover::sora_actor`); the probe's copy could be the last room's as a room loads, and
  his new model was refused (KH1's animations on entering a room, "sometimes").
- Logs: the last five runs are kept (`kh1-kh2.prev.log`, `.prev2` .. `.prev5`); the user's
  sessions were lost to two quick restarts with one kept. Reported, not yet understood: after
  sliding off a Traverse Town roof (KH1's own slide; the command menu greys), KH2's jumps stay
  low until a dodge roll (KH1's mover reads none of its jump fields `+0x3B0..+0x3D0`; the
  `ceiling:` log lines will say if it is our ceiling rule).

### 2026-10-18: Stage 151 (game volume, full ability lists)
- KH2's sounds follow the game's volume sliders: KH1's global settings (`0x14012AC50` returns
  them; Steam's build 4 bytes later) hold master `+0x10` (the device's, `0x14011BE4A`) and
  channels 1..3 `+0x12/+0x14/+0x16` (0..10; `0x1400DDD00` sets each channel to slider / 10), in
  the options menu's order: music, sound effects, voice. KH2's effects play at master × effects,
  its voices at master × voice, read when each starts (`kh2sound::game_volume`).
- A full ability list (the user's level 100 save had no Aerial Dodge): `growth::make_room` goes
  on past the least-missed list to any KH1 ability that's off (never `KEEP`: Guard, Dodge Roll,
  Combo Plus / Air Combo Plus, the specials, the abilities that are KH2 moves), then to the
  least missed even when on. Each still noted in `kh1-kh2-removed.txt`.
- Traverse Town's district doors ("curtain rooms"): KH1's animations after them, the room
  entry's refused model (stage 149's fix).

### 2026-10-18: Stage 152 (rooftop jumps; a tester's s148 log)
- Low jumps after a roof (the user's video; the tester's log: "ceiling: rose 0.0 of 7.5 at height
  620.1 (collision flags 00000009)" ten jumps running; the user's own logs the same at 457.2):
  KH1's collision bit 3 (`+0x2D0` = 9) is a slope's surface under him, not a ceiling (real
  bumps: flags 1, a partial rise). KH1's air mover pushes him along the roof rather than up;
  our ceiling rule took each jump for a head bump (cut it, locked air moves). With bit 3 the
  rise is KH2's own and no ceiling is called.
- The tester's log also showed: Sora's model refused mid-battle against the probe's stale actor
  ("drawn 872 away"), then "another body" and KH1's moves (stage 149's party-slot actor fixes
  it); spells waiting and dropped after 1.5 s (stage 149's 5 s); Aerial Dodge MAX not learned on
  a full list (stage 151). Pad records now counted once each.

### 2026-10-18: Stage 153 (no Zantetsuken, keep EXP Zero)
- The user: "remove zanteksukan, keep exp zero, increase the ability slot limit, have aerial
  dodge be its own ability still". Zantetsuken (KH1 id 59) is taken out of Sora's list like
  Superglide (the mod has no use for it; its level-up line is not shown), which frees a slot on
  a full list for Aerial Dodge (its own ability, id 67). EXP Zero (64) is in `KEEP` and no
  longer in `make_room`'s least-missed list; one taken out by stage 151 is given back (off).
- The list itself can't grow: KH1 keeps 48 bytes per party member at `+0x44`, and the next
  member's record starts `0x74` after this one (the save's layout, not ours).

### 2026-10-18: Stage 154 (the version in the Config menu)
- The user's friend: rooms breaking and no Aerial Dodge on a save that gets it here (this PC's
  s153 log: "Genie Jafar beaten: Sora learns Aerial Dodge MAX", AerialDodge motions after),
  matching the s148 build his earlier log came from. The user asked whether our addresses are
  absolute: they are RVAs (KH1 is loaded wherever ASLR puts it, `0x7ff6...` here), so they are
  the same on every PC with the same game build; an unknown build turns the mod off.
- `VERSION` (`lib.rs`, "s154") is shown on the second line of the Config menu's Camera Style
  help ("KH2 Combat version s154") and in the log's first line. `tests/version.rs` keeps it
  equal to the latest stage here: bump it with each stage. The README says how to check it,
  and that an update needs a Build again.
- Rooms in the log (the user: "the log should tell you whats happening when you enter a new
  room"): KH1's world and room (u32 at `0x142340E5C` / `0x142340EC4`; Steam `+0x233FE84` /
  `+0x233FE8C`, read from each build's `0x1402AFC10`, the flight-room check; KH-1FM-AP-LUA uses
  the same) are watched each frame: "room: world W room R entered" with who has Sora (KH1's
  reason when it does, his state, flags, a script), and 3 s on, how many of those frames KH2
  had him.
- Off a pole / bar (the user: KH1's falling animation after jumping off a pole): a KH1 move
  (`HANDOFF`) that ends in KH1's plain fall (state 2, above the floor, dropping; the log: bar
  state 15 -> 16 -> 2 with motion request 6, KH1's fall, -> 7 its landing) is KH2's from there:
  `Character::carry_into_air` starts KH2's fall at the speed KH1 had him moving (his position's
  change per frame, capped), so the swing's momentum and the fall's speed carry on.

### 2026-10-18: Stage 155 (footsteps, three moves' effects, landing hold, falls after spells)
- Footsteps mid-animation (the user): KH1 keeps Sora in its walk / run slots (5..7 in battle)
  whenever the stick is held, so its own footstep sound (`0x140293C70 (actor, kind)`, the
  floor's sound from `footstep.bin` by world `0x142340E5C` and material; Steam `0x140295E00`)
  played under KH2's attacks and rolls. Hooked: Sora's steps sound only while KH2 walks or runs
  him (or KH1 has him).
- Guard Break (165: effect elements 31, 30, 32 at frames 24, 28, 30), Finishing Leap (167: 33
  at 2, 34 at 28) and Aerial Recovery (40: element 0 at 0) join `kh2fx::DEFAULT_MOTIONS`;
  Aerial Recovery joins the sounds too. The sound log now names each element's sound, or says
  an element has none (Finishing Leap's sound, if not its elements', is still to find).
- Landing (the user's KH2 video: touchdown, ~16 frames of the landing crouch at 60 fps, then
  the run): `LAND_HOLD` 14 frames standing still before the stick moves him; buttons still from
  the reaction window. `docs/movement-model.md` corrected.
- Falling after KH1's magic in the air (the user: KH1's fall animation): KH1 drops him in its
  air state 1 there, not 2; once the spell is done (he can act) and he drops, KH2's fall as
  for stage 154's bars. KH1-move states are logged as they change.


### 2026-10-18: Stage 156 (spells in the air)
- s155 broke air spells (the user: "it cancels the spell and goes into the KH1 falling"). The
  log: a spell in the air (command 0x2 "in the air", KH1 motion 0x40000006) runs in KH1's air
  state 2 from its first frame, slot 80 the cast, sinking; stage 154's rule took state 2 +
  dropping as "the move ended in its fall" 10 frames in, set KH1's state to 0 mid-cast, and
  KH1's spell action then kept him (its fall slots 9..11) with KH2 never taking over. Now a
  command's fall is KH2's only once KH1 lets him act again (`0x1402956E0`), state 1 or 2; a
  grab's (bar, pole, ledge) as before. `release` logs a new reason even while KH1 already has
  him (that case left no line saying why).

### 2026-10-18: Stage 157 (falls handed over for real, air casts, pauses)
- Stage 154's handed-over falls never reached KH2 (the user's s156 log, a loop: "KH1's move
  ended in its fall: KH2's fall", then at once "KH1 has Sora (KH1's own action, slot 10)"):
  KH1's fall slot (10) is still up on the frame after, and `takeover` took a slot past 7 on a
  KH2 that wasn't airborne yet (no KH2 frame since) for KH1's own action. A fall just handed
  over (`AIR_START` / `HANDOFF_FALL`) counts as airborne, and KH1's jump / fall / land slots
  8..13 with KH1's ground state are leftovers, not KH1 actions (also: no more KH1 landing and
  idle before KH2 after a KH1 move).
- Air spells: KH1 keeps its cast pose and won't let him act until he lands (can-act false all
  the way down). A command in KH1's air state 2 whose current motion (not 8..13) has played its
  length (KH1 Sora's motion table, `pose::slot_frames`, at least 20 frames) counts as done:
  KH2's fall from there.
- Pause mid KH2 move (the user: resumed in a KH1 animation): `pose` drew KH2's last pose only
  while under 0.15 s old, so after a pause it was stale and KH1's slot was drawn; it is also
  used while Sora's last physics frame was KH2's (`takeover::kh2_frame`, game frames).
- Guard Break / Finishing Leap effects "look different" (the user's KH2 videos): offline
  renders (`fx_render` scenes guard_break, finishing_leap, aerial_finish, aerial_recovery
  added) show kh2-fx skipping programs 115 pppDrawMatrixWood, 120 pppRyjDrawMatrixWorld, 174
  pppVertexAp (and 49 pppVtMime for Aerial Finish): KH2's blade ribbons and Finishing Leap's
  white burst. Being implemented separately.

### 2026-10-18: Stage 158 (the guard's own motion)
- The user: our guard was "the animation he uses when somebody actually hits his guard" (with
  a KH2 video of the real guard). KH2's action button, standing, tries combo row 11 (exe
  `0x1403A9020` from the handler `0x1403A75A0`); in Sora's PTYA set row 11 is motion 173 (A322):
  the Keyblade's guard window ATKP 161 (refact self 1: a block starts the repel) frames 6..37,
  its trail 8..37, a swish sound at 1. Motion 42 (A321), which we played, is the repel (and
  Counterguard's window, row 39 while 42 plays: the same handler checks motion 0x2A). The press
  now starts row 11's motion (`motion::GUARD_PRESS` 173 as a fallback); a block still plays 42.
- `Output::weapon_trail` (the motion's trail window): KH1's Keyblade trail shows during the
  guard's swish; 173 joins the KH2 sound moves (its swish).

### 2026-10-18: Stage 159 (no KH1 high jump after its moves)
- The user (a video): whenever KH1 plays one of its animations, a jump on the first frame Sora
  can act is KH1's high jump. The log: an air spell, KH1 lands him (state 0, can act false),
  then state 1 with can act true at the floor (KH1 jumped), state 2, and "KH1's move ended in
  its fall (290 above the floor)". During `HANDOFF` the control hook left KH1 its jump / Square
  bits (KH1's own moves use them: jumping off a bar). Now once KH1's move has him standing and
  free (state 0 and KH1's can-act test, `kh1_move_done`) they are KH2's: stripped from KH1, and
  a jump press then ends the handoff with KH2 jumping at once (`JUMP_NOW` clears the held flag).

### 2026-10-18: Stage 160 (jump out of the landing hold)
- The user: the landing hold is right, but a jump should end it. A jump press now opens the
  landing's cancel at any frame (other buttons still wait for its reaction window, frame 8).

### 2026-10-18: Stage 161 (Guard Break's and Finishing Leap's full effects)
- kh2-fx gains the four programs those moves used and we skipped (read from KH2's exe;
  `docs/kh2-effects.md` §6, §7.3, §7.4, §12.1): 115 pppDrawMatrixWood (an upright billboard,
  turning to the camera about the effect's own y: Guard Break's spiral around the thrust),
  120 pppRyjDrawMatrixWorld (an instance keeps the root it was spawned at: Guard Break's light
  fans stay put while he lunges), 174 pppVertexAp (VertexApLc with the vertex taken through the
  spawner's matrix: Finishing Leap's five flame spikes on a ring around him), 49 pppVtMime (a
  vertex morph between two models: Aerial Finish's rings widening). Offline renders
  (`fx_render`, new `--flip-z` / `--follow-root` options) skip nothing for guard_break,
  finishing_leap and aerial_finish; explosion and magnet render byte-identical.

### 2026-10-18: Stage 162 (jump casts drawn as KH2's, camera, Guard Break's spiral)
- Jump casts still showed KH1's animation all the way down (the user; the s161 log: a spell in
  the air stays in KH1's air state 2 with its cast slot 80 until he lands, so stage 157's "cast
  played through" never came before the ground). Now drawn, not taken: while KH1 has him for a
  spell cast in the air (`takeover::air_cast`, the spell from the command's second byte: Fire 0
  .. Aero 6), `pose` plays KH2's air cast for that spell (MAGC air motions 58 / 61 / 64 / 67 /
  70 / 73; Gravity Magnet's, Stop Fire's, Aero Reflect's), then KH2's fall, and KH2's landing
  once KH1 has him on the ground. KH1 still moves him and runs the spell.
- Out of battle, KH1's hold at the end of such a landing counted as a "scene" and gave the
  camera to KH1's (the user: a jump cast of Fire zoomed the camera in). Holds while or within
  1.5 s of one of KH1's moves no longer count.
- Guard Break's effect sat around the middle of the Keyblade (the user). Its spiral (element
  32) is 235 along -z of KH2's object root; our root frame mirrored the model's y only, which
  keeps the model's forward (+z) and so put the spiral behind him. The root frame is now the
  model's turned half round about x (y and z negated); centred elements (Explosion, Magnet
  Burst) are unchanged.

### 2026-10-18: Stage 163 (KH1's jump casts, landing hold after them, L1 spam)
- The user: KH1's magic animations after all (stage 162's KH2 air casts taken out), but the
  landing after a jump cast should have KH2's landing hold, and a jump out of it KH2's jump.
  When KH1 lands him out of an air cast (`air_cast`, KH1 state 0 on the floor), KH2 takes him
  at once in its landing (`Character::land_now`: the 14-frame hold, a jump at any frame); KH1's
  cast action flag (`+0` bit 4) is cleared so its can-act test lets KH2's input through.
- Spamming L1 and jump launched Sora in KH1's animations (the log: KH2's jump, then KH1 state 2
  in its jump slot 12 under KH2, "KH1's own action, slot 12/13", state 21). The control hook
  left KH1 its jump whenever the raw pad had L1 (KH1's shortcuts); KH1's own button state can
  differ from the raw pad for a frame, and then saw a plain jump. The strip now follows L1 in
  KH1's own state (`0x40000`, its pad table `0x140528F90` maps the raw `0x400` to it).

### 2026-10-18: Stage 164 (jump held, then L1)
- The user: holding jump and pressing L1 "does crazy stuff". The log: KH2's landing with KH1
  in its state 2 and jump slot 12 (flags 0x48003), then KH1's own air moves (state 21). With
  L1 down (KH1's shortcuts) the control hook left KH1 all its button bits, the held jump too,
  and KH1 jumped him as KH2 landed. In the shortcut menu (or a submenu) only KH1's "pressed"
  bits stay now (a shortcut, backing out); the held and repeat jump / Square bits are KH2's.

### 2026-10-18: Stage 165 (air casts play through; KH1's own jump under KH2)
- The user (a video): Fire in the air "isn't playing the full animation and looks weird": the
  log showed stage 157's "cast played through" handing a jump cast to KH2's fall mid-air. Air
  casts (`air_cast`) now stay KH1's to the ground, then KH2's landing (stage 163).
- Holding jump and pressing L1 still launched him (the s164 log: "L1, Circle" while KH2 had
  him in the air, then KH1's jump motion (slot 9, set from its own state step, not the control
  step we strip around), and KH1 state 2 / slot 12 as KH2 landed). Wherever it comes from: when
  KH1 has put him in its jump (state 1 / 2, slot 8 / 9 / 12 / 13) on a frame KH2 had him and
  no handoff ran, KH1's state goes back to 0 and KH2 keeps him (logged).

### 2026-10-18: Stage 166 (no jumping through Olympus's roof)
- The user: in the Coliseum's lobby a jump went through the roof. The log: "rise on a slope's
  contact (flags 9)": that ceiling carries KH1's collision bit 3 too, and stage 152's roof rule
  let KH2's rise through. Bit 3 now counts as a roof only when it was there from the first 3
  frames of the rise and unbroken since (`RISE_FRAMES`, `SLOPE_RUN`): taking off from a roof,
  not meeting a ceiling on the way up.

### 2026-10-18: Stage 167 (attack out of a spell's end frames, combo carried on)
- The user: KH2's magic has end frames an Attack cuts into, going on with the combo. KH2's
  ground casts open their range-14 window about 8 frames after the magic-cast event (Fire 56:
  cast 22, window from 30 of 80; Blizzard 59: 16 / 24 of 72; Thunder 62: 18 / 26 of 68).
  For KH1's casts (`HANDOFF` from a command): an Attack pressed meanwhile (`ATTACK_AFTER`)
  now ends the handoff only once the spell is out (`spell_out`: a new attack object owned by
  Sora in KH1's pool, `combat::sora_shots`, or 55% of KH1's cast motion for spells with none),
  not before (before, an Attack 10 frames in could cut the cast itself). KH1's action flag is
  cleared there, and KH2's combo state from before the spell carries over
  (`Character::combo_state` / `set_combo_state`: the next swing counts on from it).

### 2026-10-18: Stage 168 (animation-cancel KH1's spells)
- The user: "animation cancel the KH1 spells". Once the spell is out (`spell_out`), not only
  an Attack (stage 167) but a jump, Square (roll, guard, glide) or the stick ends KH1's cast
  and hands Sora to KH2 at once (KH1's action flag cleared; logged "spell's end frames
  cancelled by ..."). Before it is out, Square and the stick no longer end the handoff either
  (they could cut the cast 10 frames in). Only an Attack carries the combo on.

### 2026-10-18: Stage 169 (pausing mid KH2 move, again)
- The user: pausing during KH2 animations still put him in KH1's. The s168 log: paused while
  falling (KH2's Air), resumed, "KH1 has Sora (Sora is in another body)", KH2 restarted
  standing in mid-air, KH1's fall slot took him (state 21, its air moves). `sora_body_seen`
  timed Sora's drawn model by the clock; during the pause only the menu's copy is drawn, far
  from his actor. A gap of more than 250 ms in Sora's physics frames marks a resume
  (`takeover::resumed_within`), and for 800 ms after one his body counts as seen.

### 2026-10-18: Stage 170 (Reflect in the Magic menu)
- The user: "lets add reflect into the magic menu, i want the sound and VFX". KH1's command
  records (`0x14028D7D0 (id)`, 16 bytes, 256; 0x7D.. empty) get 0x7D: magic, spell byte 7,
  word index 0x1FF (named "Reflect" by pointing the name drawer's `0x14028D820` call at
  `0x1402787FB` to ours). After KH1's battle-menu fill (`0x140292330`) it goes into the magic
  list (`0x140529638`) before Summon once Sora has Aero (`reflect::learned`), greyed as KH1's
  spells are with Fire's cost (`0x1402A2A80`; spell data `0x142D23598 → +0x30`). The list's
  layout (`0x142859E60`: `+0x10` / `+0x28`) gets 8 row panels for 9 entries (KH1 builds 7).
- `combat::command` hands command 2 / spell 7 to `reflect::chosen` (KH1 never runs it). KH2
  casts it when its move allows (MAGC: 71 / 72 / 73), pays MP (`0x1402A39D0`), and runs
  `magic/REFLECT_1.mag`'s script with `kh2-magic` (trigger 20, release at frame 0). While ATKP
  573 lives, `hurt` stops Sora's hits (no-effect) and counts them. The counter (574 every 10
  frames at level 1, then the 667 burst) lands through `Strikes::spell_strike`.
- The spell's PAX is drawn beside Sora's package (`kh2fx`, textures appended), its elements'
  sounds play (`kh2sound`), cast voices only with `kh2_voice`. ini `reflect = on`. Not in the
  shortcuts (KH1's shortcut slots hold spells 0..6). `docs/kh1-reflect-spell.md`. Built by a
  helper session from s166 and merged over s167..s169.

### 2026-10-18: Stage 171 (no cancelling jump casts)
- The user: "you can animation cancel out of KH1 jump spells, so no good there". Stage 168's
  end-frame cancels (Attack, jump, Square, the stick) now apply to ground casts only; a cast
  in the air (`air_cast`) plays KH1's animation through to the ground and KH2's landing hold
  (stage 163).

### 2026-10-18: Stage 172 (Reflect in the shortcuts, Reflera / Reflega, 1 MP, held barrier, effects under the pause menu)
- The user: "the reflect vfx pass through the pause menu, make reflect able to be put in the
  shortcuts, also include the two upgrades. Reflect should cost 1 MP", then "can we have
  reflect keep shielding until attacks have stopped". Built by a helper session from s170.
- Pause menu: KH1's pause menu task (`0x1402D3040`, priority 159999, opened by `0x1402D3450`)
  draws its windows before the world pass and repoints the camera for the party models, so our
  effects drew over it. `kh2fx` draws nothing while KH1's pause state `0x14232E9B4` is non-zero
  (1..6) or Sora has had no physics frame for 120 ms (`takeover::paused`).
- Shortcuts: only KH1's battle fill reads the slot bytes and indexes its spell table by them,
  so Reflect is never stored there: its slots keep KH1's none (`0xFF`) plus a bit in `+0x847`
  (bits 4..6; the fourth's nibble is 8). After the fill those entries are command 0x7D (the
  menu's path, `reflect::chosen`); KH1's learn sees them as taken.
- Customize: the spell list (`0x142E9A9F0`, 8 ints) moves to a buffer of ours (its four `lea`
  repointed) so Reflect fits before "none"; 9 rows 25 apart. Its help line (`0x14030028E`) and
  name calls (`0x14028D100`) are ours. Works with `shortcuts = 3` too.
- Levels: `reflect::level` = Aero's level (KH1's spell table `+0`, else save `+0x492 + 6`);
  each level its own `REFLECT_n.mag`, MAGC row and sub; Reflect / Reflera / Reflega.
- MP: KH1's MP is whole points; a cost ≥ 100 is hundredths (check `cost/100 <= MP`, pay
  `cost/100`), so Reflect costs 100 (1 MP) at every level.
- Held barrier: from the barrier's 22nd frame, while the last blocked hit is ≤ 20 frames old
  (cap 5 s), `kh2_magic::Magic::hold` freezes the cast (script and effect ages; its attacks
  stay, so `hurt` keeps blocking); the counter fires after; the barrier effect is restarted if
  it ends. Tests `reflect_counters_what_it_blocked`, `reflect_barrier_held`.

### 2026-10-18: Stage 173 (glide in place)
- The user: "In KH2 you're able to glide in place". The glide's speed now follows the stick's
  tilt (`(1 − a)·glide_speed·tilt + a·|v|`): with the stick let go he slows to a hover where he
  is, still descending to and settling at `glide_fall_height`. The exact KH2 scaling is not
  traced in the exe (hypothesis in `docs/movement-model.md`).

### 2026-10-18: Stage 174 (Reflect falls in the air)
- The user: "make sora fall while casting reflect mid air, just like kh1 magic". KH2's air cast
  (motion 73) hovers in its no-gravity window (range id 3, frames 16..74). `Character::
  air_magic_falls` (the hook sets it) skips that window for magic actions: the cast falls from
  rest as it plays; landing keeps the cast going on the ground.

### 2026-10-18: Stage 175 (Reflect spammed: one counter at the end)
- The user: "reflect needs to not do the burst damage until the player stops spamming it".
  A Reflect cast while the last one is still in flight (barrier, hold or counter) now ends that
  one at once (`kh2_magic::Magic::cancel`: scripts stop, effects killed, attacks freed) without
  its counter, and the hits its barrier counted carry over to the new barrier
  (`Runtime::carry`, added to its reflect count once it stands). Only the last cast of a chain
  fires the counter and burst, for every hit blocked along the way.
- Stage 174 (Reflect falls in the air) went out in the same build.

### 2026-10-18: Stage 176 (magic in the combo's queue)
- The user: "reflect and the other magic should be part of the combo cue, so it can get cast
  and the next attack waits for the magic to get done", and "reflect should have a finisher
  like in KH2". KH2's casts (Reflect): an Attack pressed during the cast is buffered
  (`buffered_attack`) and taken when the cast's reaction window opens or at its end; a cast
  started mid-combo leaves `combo_carry`, so that swing goes on with the combo. KH1's casts:
  an Attack waiting through one (`ATTACK_AFTER`) now carries the combo on at the handoff's end
  too (`COMBO_ON`), not only when it cut the end frames.
- Reflect's finisher was already there (KH2's rule, `Character::cast`: cast in a combo-magic
  window when the next swing would be the combo's last, MAGC finisher motion 72, sub + 1);
  the user's logs only show casts from free movement (shortcuts), where it can't happen.

### 2026-10-18: Stage 177 (Horizontal Slash in the combo; air casts land into KH2's landing)
- The user: "Horizontal slash can be spammed forever, this is bad". A forced Square move row
  (Upper Slash 12, Horizontal Slash 38) only checked the row's own conditions, and a normal
  row has no count check, so Square kept taking Horizontal Slash (a jump row: each one hops,
  so Sora stayed up). Now a non-finisher Square move needs `count < max` after the count
  update: it is one swing of the combo, and at the end Attack's finisher is next.
  Test `horizontal_slash_is_part_of_the_air_combo` (24 slashes before, 1 in a combo of 3).
- The user: Final Form's air Reflect falls and lands during its animation; mimic it for base
  Sora and end in KH2's landing. The fall was s174 (`air_magic_falls` skips motion 73's
  no-gravity range); now a falling air cast touching the ground goes into the Land action
  (with its 14-frame hold, jump-cancelable) instead of standing out the rest of the cast.
  The barrier and its counter don't watch the caster's motion (kh2-magic test with the cast
  motion ending at frame 8: same barrier and counters), so landing early keeps Reflect up.
  Test `falling_air_cast_lands_into_the_landing`.

### 2026-10-18: Stage 178 (glide hover, Aerial Dodge, Reflect's sounds, Fire's cancel, magic in the combo)
- "KH2 sora has a animation for gliding in place": KH2's ACTION_GLIDEHOVER (exe `0x1403A8F44`:
  a glide started with the stick neutral is action 0x11, else 0x12; `0x140422B20` plays motion
  208 and goes to the glide on the stick; `0x140422930` slows it ×0.9 a tick). Ours: the same,
  and a glide let go hovers once slowed under 2 per frame. `motion::GLIDE_END` (208, unused)
  is `GLIDE_HOVER`. Test `glide_in_place_hovers`.
- "aerial dodge doesnt push you forwards unless you're tilted forwards in KH2": neutral, the
  dodge has no horizontal speed (was `air_dodge_speed` along the facing). Test
  `neutral_aerial_dodge_goes_straight_up`.
- "These are the wrong Reflect sound effects": se000's id records are 68 bytes and the u16
  read as the entry (end - 4) is only a running count there; the entry is the last u16 (end -
  2), equal to it in se999 / se500 (20-byte records, the sounds checked before). Read that way
  every spell's ids land in one block (Reflect's on entries 180..187) and 12 of 124 ids are
  unplayable (57 before; Reflect's 138 / 139 / 176 played nothing). Fire, Magnet Burst's 169
  and the other se000 sounds change too. `docs/kh2-sound.md`.
- "can you make it so fire cant be cancelled out until fire actually shoots the projectile":
  `spell_out` no longer falls back on 55% of KH1's cast motion for spells with a shot (only
  Cure / Stop); the others wait for their shot (or KH1's whole cast motion). The shot test
  compares with the fewest attack objects seen since the cast (a swing's hitbox out at the
  handoff ended during it, so Fire's shot came back to the start count). Logs "command's spell
  Some(n) out after t s (its shot / KH1's cast motion's time)".
- "casting a spell mid combo ... should essentially add onto the main combo / it should insert
  itself" (video: Reflega mid-air combo, then a whole new combo): a spell is mid-combo while a
  swing plays or one just ended (in the air until landing, on the ground for 60 frames), not
  only during a swing (`combo_live`); it takes a step (counted as `try_attack` counts: hit,
  Combo Master or in the air) and the combo goes on after it. KH1's spells too
  (`Character::magic_step` at the command handoff, `SPELL_COMBO` restores the combo after).
  Test `spell_mid_combo_is_a_step_of_it`.

### 2026-10-18: Stage 179 (revenge value, Reflect's invulnerability, Attack cuts air casts)
- "multi-hit attacks (like explosion) should only have the last hit towards revenge value". KH1
  has no revenge value in its engine (traced by a helper session: no counter; a hit makes a
  record queued on the foe, `+0x3EC` linked by handles at `+0x30`, with the foe's hit flags
  `+0x3E0` and serial `+0x3E8`; a foe with a script (`+0x374` bit `0x10000`, machine `+0x144`)
  hears of it at once, event 2 from `0x1402BCF20` in the contact, and again when it takes its
  queue, event 0 from `0x1402B59F0`; its script applies the damage and counts hits as it
  likes). KH2's own data marks the hits: revenge value 0 on Explosion's ticks (911) and 24 on
  its blast (912), Magnet Burst 1901 / 1903, Aerial Finish 915..917 / 927. So a hit whose row
  has none while one with some is still to come in the motion (`revenge_to_come`) is landed
  silently on a scripted foe: script flag off for the contact, the record taken back off the
  queue and freed, flags and serial restored, its whole damage carried (`CARRY`) to the foe's
  next hit, normally the move's last. Unscripted foes as before. Logs "silent hit on scripted
  foe". The ticks' flinch is the script's too, so scripted foes no longer flinch to them.
- "while the reflect animation is up sora should be invincible all the way through": enemy hits
  in any of Reflect's cast motions (MAGC ground / air / finisher of each level) are evaded
  (`hurt`), before the barrier stands and after it falls; the barrier's own still count.
- "magic transition into the base attack during a combo" (asked: "Attack cuts the spell
  short"): an Attack once a KH1 spell is out now cuts air casts too (stage 171 left air casts
  uncancellable; jump, Square and the stick still don't cut them), into KH2's next combo swing.

### 2026-10-18: Stage 180 (magic out of a swing at once; Reflect chains in a combo)
- "too much a pause before magic gets cast in a combo" and "make it possible for attacks to
  cancel into magic": Sora's swings have no magic window (only the casts have `ComboMagic`), so
  `can_cast` was false through a whole swing and a spell chosen mid-swing waited for its end
  (0.15..0.7 s in the user's log). A swing now takes a spell in its combo or finisher window,
  when it would take its next swing (right after its hit). Test
  `spell_cast_from_the_swing_combo_window`.
- "if sora does a combo with reflect, any attack done to every reflect should burst out of the
  final reflect in the chain": a Reflect that blocked something waits with its counter (held,
  barrier hidden and stopping nothing) while the combo has steps left (`Output::combo_live`);
  a later Reflect in the combo takes its hits over (s175's chain), the combo's end lets the
  last one burst for all. `docs/kh1-reflect-spell.md`.

### 2026-10-18: Stage 181 (no footsteps between swings)
- "i can still hear the footstep sound fx while walking forward in the middle of a combo":
  stage 155 let KH1's footstep through on any frame KH2's state was Walk / Run; between two
  swings with the stick held KH2 is in its run for a few frames (still showing the swing, the
  log's "Run (motion 166)"), and KH1's run cycle stepped then. Now only after `STEPS_AFTER` (10)
  frames running KH2's own walk / run motion (1 / 2).

### 2026-10-18: Stage 182 (Reflect chains burst for real; the burst's sound)
- "if reflects are cast into one another, basically delay the burst until the last reflect, all
  attacks and triggers should burst at the last reflect". The user's s180 log: "cast again in
  flight ... 2 blocked hit(s) carried over", "2 carried hit(s) counted on the new barrier
  (count 4)", then "2 hit(s) blocked, 0 counter hit(s)": `Magic::cancel` left the cancelled
  cast's attacks standing until the next `update`, so the carry went onto the old barrier
  (still found first) and the new one countered for nothing. The chain (s175) never burst.
  `cancel` now frees the cast's attacks at once (kh2-magic test
  `reflect_chain_counters_from_the_last_cast`, which fails without it). The carry is also
  every hit of the chain so far (`rt.blocks`), not only a standing barrier's count, so a Reflect
  cast into a counter already under way carries its hits too.
- "the burst sound isnt in the game either": the script starts no element for the burst
  (ATKP 667 / 668 / 669); KH2's own hit effect for it (`trap_attack_set_pax`) isn't modelled,
  and element 11's sound (174) plays nothing in the PC files. Element 7's sound (173, shared by
  all three levels, 1.9 s) now plays as the burst attack appears. Inferred, not traced.

### 2026-10-18: Stage 183 (the user's magic-combo spec; aerial landing, submenu guard, roll-off dodge, dodge momentum)
- The user's written spec (magic in the combo string, KH2 Reflect finisher rules, KH1 magic
  integration, three combat bugs) and a bug report (Aerial Dodge momentum). In the controller
  one rule for a spell's combo slot (`spell_slot` / `take_spell_slot`, `SpellSlot`), used by
  KH2's casts (`cast`) and KH1's spells (`magic_step`): a new combo's first step, the next
  step of a going one (its finisher on the last), or during a finisher only the next finisher
  while Finishing Plus leaves one. `can_cast` follows it (a finisher's window only for the next
  finisher; nothing after the last finisher until it ends). A finisher cast (`cast_finisher`)
  takes no cancel except the next finisher in its finisher window, and ends the combo; a
  finisher swing's finisher window cancels only while another finisher may follow.
  `try_attack` takes a finisher cast (or a spell that left `carry_finisher`) as the finisher
  playing. `ComboState` carries `finisher_next` over KH1's casts. Tests: `reflect_cannot_
  follow_the_last_finisher`, `finishing_reflect_as_the_first_finisher_then_a_finisher`,
  `finishing_reflect_twice_then_the_combo_ends_after_it`, `finishing_reflect_as_the_second_
  finisher`, `reflect_mid_combo_is_one_step`, `kh1_spells_take_one_slot_each`.
- KH1 spells in the hook: the slot is logged ("spell cast: combo slot n") and decides what may
  cut the cast once the spell is out (`SPELL_SLOT`): an Attack or another spell (the combo's
  next step) unless it was the combo's last slot; a jump or Square only out of a step on the
  ground; never the stick (spec rule D). KH1 runs no command during its cast, so an Attack or
  a spell chosen then is taken off its command record (`combat::take_chosen_command`; the user:
  no attack out of magic); a spell waits in the queue (`MAGIC_AFTER`, `resume_command` keeps
  it through the cast). After the cast the combo goes on from the spell's slot (not after the
  last). A Reflect barrier waits for the combo only with something else in it (count ≥ 2).
- Spec 3.1, aerial attacks to idle: the user's logs, "Attack 181 at 85 above the floor -> Run",
  "Attack 192 -> Idle": landing mid air-attack went to Free; now KH2's Land. Test
  `air_attack_lands_into_the_landing` (fails without).
- Spec 3.2, guard from a submenu: with a command submenu open the pad mask and the control-step
  strip left Square to KH1, whose own guard works in the air. Now only the L1 shortcut menu
  leaves KH1 Square; a submenu keeps only its back-out (jump / cancel), Square is KH2's (its
  guard ground-only).
- Spec 3.3, no Aerial Dodge after rolling off a ledge: `air_dodge_ready` came only from a jump;
  a roll or Quick Run taking off (`since_roll`) gives it too. Test
  `aerial_dodge_after_rolling_off_a_ledge` (fails without).
- "Aerial Dodge momentum is reset ... when the player presses a directional input": air control
  steered toward the run speed (8) at `air_keep` 0.9 a tick, draining a dodge's 18..36 a frame
  in a few frames, while letting go only applied `air_drag`. Now the stick steers without
  fading the speed faster than `air_drag`, unless pushed against. Test
  `dodge_momentum_kept_with_the_stick` (fails without).
- Review (a separate session, against the spec) found three more, fixed: a finishing spell
  could follow a finisher that missed (`spell_slot` and `can_cast` now need the finisher to
  count, as `try_attack` does; test `no_finishing_reflect_after_a_missed_finisher`); an air
  spell's combo (its finisher carry) survived the landing of Reflect's air fall (`land()` resets
  the combo there, as for aerial swings; test `air_spell_combo_ends_on_landing`); a spell chosen
  after KH1's cast had ended waited in the queue until the stick or a button moved
  (`kh1_casting` now needs Sora still unable to act, and a spell waiting once the cast is done
  ends the hand-off). Known limit: KH2's air cast motion (73) has no finisher window, so after
  an air finishing Reflect the next finisher can't follow in the air.

### 2026-10-18: Stage 184 (multi-hit moves flinch again)
- "Multi-hit moves should continue triggering the enemy's taking-damage animation on each
  appropriate hit, while revenge value is only added on the finishing hit": s179's silent hits
  never reached the foe's script, which is what plays its reactions (its reaction calls run
  KH1's engine reactions, `0x1402B9B60` → `0x1402B62C0 (actor, record)`; the default path
  `0x1402BCD30` only changes HP). Now a silent hit on a foe already in one of its damage motions
  (`+0x164 - +0x3EA` under 10, KH1's own test in `0x1402B62C0`) runs that engine reaction itself
  (and `0x1402B65F0` first for records with `+0xC` bit `0x800`, as the call does) before the
  record is freed; a foe not reacting hears the move's first hit as usual (its script decides
  whether it flinches: super armour stays), the rest stay silent (`TOLD`, per move and foe).
  So a multi-hit move counts at most twice toward a script's hit count (its first hit when the
  foe wasn't already reeling, and its last), never once per tick; forcing a flinch the script
  didn't choose risks breaking bosses (the Cloud kind of bug). Damage still carried to the last.

### 2026-10-18: Stage 185 (Keyblades teeth up; hair, hood and keychain physics)
- "sora holds his keyblade backwards, with the teeth of the keyblade facing the ground", "every
  keyblade is held backwards": the grip bone (280) hangs from the hand as in KH1's rest pose,
  whose roll of the blade isn't KH2's. The blade and keychain lie on the grip's x (keychain
  bones at -21..-56 on it in the bind), so every Keyblade (not 0x52..=0x55, the dream weapons
  and the wooden sword, nor `flip_weapons`) is turned half round about that axis with its
  subtree (`pose::keyblade_teeth_up`; ini `keyblade_turn = on`).
- "sora's keychain, hair, hood etc dont have wind physics during any KH2 moves": KH1 has none
  either (each of its motions animates them by hand), so under KH2's motions they stayed rigid.
  `kh1_data::jiggle`: per chain, particles in world space pulled back toward the pose, with
  gravity and a breeze, kept at the bones' lengths, each bone turned at its simulated child
  (subtree with it): the 14 hair spikes under the head (151; root, joint, tip each), the hood
  80..84, the keychain 284..292. Reset after half a second without KH2's pose or on a warp.
  ini `physics = on`. Tests in `jiggle.rs` (still pose stays, a carried chain trails then
  settles, a warp restarts it). Tuning is a first guess: not yet seen in game.

### 2026-10-18: Stage 186 (physics toned down)
- The user's video of s185: "it works, but ... needs to be toned down a lot, there's a lot of
  clipping" (hair spikes splayed into each other and the head, the hood flipped up). Stiffer
  springs and more damping, no gravity on the hair and little on the hood, a breeze a tenth of
  s185's, and each bone's swing capped from its posed direction (`Chain::max_turn`: hair 10°,
  hood 12°, keychain 35°; the simulated tip is pulled back to the cap so the swing doesn't build
  past it). Test `swing_capped`.

### 2026-10-18: Stage 187 (physics steps by game frame)
- "the animations arent smooth, they kind of stutter" (s186 video). The jiggle stepped by the
  clock in fixed 1/60 s steps; frames come 13..22 ms apart (the camera's log), so some draws ran
  no step (the chains left at Sora's last place, then snapping after him) and others two. Now it
  steps once per Sora physics frame (`takeover::GAME_FRAME`, counted where `FRAME_KIND` is set);
  a draw of the same frame again re-applies without stepping; over 30 frames missed, it starts
  over. (The recordings' whole-frame repeats, 16..35% of frames, are in older videos too: the
  recording or frame pacing, not ours.)

### 2026-10-18: Stage 188 (the keychain hangs)
- "the actual movement of the keychain is good, but ... the starting position of it doesnt look
  natural, it looks like the chain is kind of floating": KH1's rest pose holds the keychain
  straight out of the pommel along the blade, and s186's pull back there (stiffness 0.18) with
  its 35° cap kept it out. Now a free pendulum (no pull, full gravity, damping 0.12, 170° cap):
  it rests hanging from the pommel. A chain starting over first settles (`SETTLE` 120 steps, no
  breeze), so it isn't seen dropping into place. Test `a_free_chain_hangs_down`.

### 2026-10-18: Stage 189 (Drive: the second command page)
- The user: "add a drive form. For now, lets add wisdom form ... press left on the dpad to have
  essentially a second command menu (like how it is in KH2,) and have a drive menu at the bottom
  of the second command menu". Asked: Wisdom costs MP; KH1 Sora recoloured blue; page 2 is
  Attack, Magic, Items, Drive. Three helper sessions researched the menu, Wisdom's data and the
  recolour; this stage is the menu (`docs/kh1-drive.md`, `win/drive.rs`, ini `drive = on`).
- KH1's top menu (menu 0) is rebuilt each frame; its fourth row isn't one of its entries but
  the magic list's last (Summon, command 6, opening menu 3). Page 2 turns that row into Drive:
  D-pad left (logical `0x80`, cleared before KH1's step, which would move the cursor to Attack)
  at the top level flips the page (not with the L1 menu, a submenu or a summon out); after the
  fill the magic list ends in Summon (appended when KH1 had none), its bit is Drive's, the top's
  Magic entry is forced in when Sora has no spell (blocked), menu 3 holds our form commands
  (records 0x7E.., magic type, argument 0x10 + form, word 0x1FE..), and word 6 reads "Drive"
  on page 2. A form chosen reaches `combat::command` as magic with that argument
  (`drive::chosen`; this stage only logs it and plays KH1's "can't" sound). The step, fill and
  name hooks are shared with `shortcuts` / `reflect` (whichever installed them calls `drive`).

### 2026-10-18: Stage 190 (the keychain hangs down)
- "the keychain is now floating upwards??" (a screenshot): gravity pulled toward the world's -y,
  but KH1's world has y pointing down (the pose's model space is y-up; `to_world` flips it), so
  the free keychain of s188 rose. Down is now the model's down taken through `to_world`. Test
  `hangs_down_in_a_y_down_world`.

### 2026-10-18: Stage 191 (Wisdom Form; lunges kept out of exits and crates)
- Wisdom Form itself (`win/form.rs`, `docs/kh1-drive.md`): page 2's Drive entry, 3 MP (KH1's
  own payment), KH2's change (Sora's Drive motion 52, invulnerable; the swap to `P_EX100_MAGF`
  loaded beside KH2 Sora, `pose::Form`, facing / lock kept; arrival 53), Quick Run LV1 and
  Sora's support abilities, 40 s of battle time (the gauge stands still out of battle), the
  entry reading "Revert" in the form, revert motion 4 on the ground, at once on a KO (`hurt`).
  Its shots: the motions' frame trigger 1 runs the model's `wisd` script (`Magic::
  object_effect`) with its effect archive as `kh2fx` package 4; the shot's unbound effect is
  placed where the script moves it (new kh2-fx `Effects::set_anchor`); ATKP 859 lands through
  the spell hit records (now returning the attack key, so the hit is passed back to the
  script); unlocked, the nearest enemy in front is the target. Reflect's sub + 10 in the form.
  Reflect's script world is shared (`reflect::world_now`).
- The blue (`win/tint.rs`, `docs/kh1-tint.md`): a hook on KH1's mesh draw `0x1401DB7F0` sets the
  mesh shader's Ambient / fog for Sora's clothes meshes (3..7: jacket, jumpsuit, trim), read
  from his model's mesh table, and puts KH1's back after; faded in over 15 frames.
- "a combo pushing you forward shouldnt bring you into a new room if you're by the entrance of
  a room, also, if a combo pushes you forward into a box, it shouldnt go into soras push
  animation" (`win/gates.rs`, `docs/kh1-exits-push.md`): exits are collision faces with an
  event number that KH1's mover queues on every touch; during takeover's move of an attack
  frame a new face event is dropped and the move undone. Crates count 12 frames of Sora's
  touch themselves; during KH2's attacks (and 20 frames after) their "touched by" field is
  cleared before the count.
- None of it watched in game yet.

### 2026-10-18: Stage 192 (Wisdom: its spells, slide, moving shots; tint and effect fixes)
- The user: "give wisdom form max quick run, KH2 sora slides when he stops in wisdom form. Also,
  lets use the wisdom form version of these spells [asked: Fire, Blizzard, Thunder, Cure]. Also,
  using the base wisdom form combo shouldnt stop sora in his tracks"; "sora's outfit isnt blue,
  and the effects are still covering sora"; "the effects last after reverting as well".
- Quick Run MAX in the form. `character-core`: `MoveParams` acceleration / brake (from prty)
  and the allow-movement range (41), tests `ground_inertia_slides_to_a_stop`,
  `attack_moves_in_its_allow_movement_range` (`docs/movement-model.md`).
- `formmagic.rs` (the shots moved there from `form.rs`): KH2's Fire / Blizzard / Thunder / Cure
  in the form (KH1's menu or shortcut → `combat::command` → `spell_chosen`; cast when KH2's
  move allows, `before_step`), their packages in `kh2fx` (handles now carry 5 bits of package);
  Cure heals KH1's HP. Test `wisdom_spells_run_at_their_sub`.
- The light on Sora was the arrival's elements (100 loops until ended): ended when the arrival
  does; everything of the form's put out on a revert.
- The tint found no table: the PC's Sora model uses the alternate mesh table (`0x1401DB4F0`);
  read now, pieces by texture index; the log lists the table.
- The gauge drains ½ a frame out of battle (KH2's), 1 in battle.

### 2026-10-18: Stage 193 (the Drive glow ends; no magic queue; revenge changes reverted)
- "the glow still covers sora in the form": Sora's Drive motion (52) starts his package's
  elements 2..13 on his limbs, looping (3..6) until ended; flagged 0x200, they end with their
  owner, which KH2 replaces at the swap. kh2-fx `Effects::owner_gone`, `kh2fx::owner_gone` at
  the swap and the revert.
- "fire and blizzard dont work in wisdom form": the cost read from KH1's spell data (`+0x30`)
  was 30 for them (3000: refused) and junk for the others; every KH1 spell costs 1 MP.
- "remove [the magic queue] entirely but ensure that magic stays connected to the base games
  combo strings": a command, Reflect or a Wisdom spell chosen during a KH2 move now waits only
  while that move plays (`takeover::KH2_MOTION` / `same_move`), going at its next combo window
  or not at all (KH2's own buffer), instead of up to 5 s. A spell chosen during KH1's cast still
  cuts it once the spell is out (the combo's next step).
- "revert this back to before we asked to make any revenge value changes": s179's silent
  multi-hit ticks and s184's engine re-flinch are gone; every hit lands as it did before.
- The tint: Sora's PC model is the remaster's (9 meshes on 4 textures, other sizes than the PS2
  file), so the clothes meshes aren't known; this build colours each mesh plainly in the form
  (0 red .. 8 orange, `tint::PROBE`) to tell them apart from a screenshot.
- "revert the recent aerial dodge change": s183's air control (the stick steering what he has,
  an Aerial Dodge's speed kept under it) is back to KH2's plain steer toward the stick's
  speed; its test `dodge_momentum_kept_with_the_stick` is gone.

### 2026-10-18: Stage 194 (Wisdom's air shots move and fall)
- "in KH2, wisdom forms aerial combat allows you to move in the air while the attack plays,
  and sora also has gravity": an attack whose motion has an allow-movement range steers in the
  air with the stick (air control as in a jump) and ignores its no-gravity ranges (181: 16..40),
  turning with the stick when nothing is targeted. Test `moving_air_attack_steers_and_falls`.
- A public copy of the KH1 mod alone was made (fresh history, `local/export_public.py`), on the PC beside
  this repo (`kh2-combat-public`).

### 2026-10-18: Stage 195 (Wisdom's blue by texture swap)
- "the outfit isnt blue": the PC draws the remastered Sora (its meshes go through `0x1401DB690`,
  never the hooked `0x1401DB7F0`), and his jumpsuit shares one HD texture (`-1c1`) with the
  jacket, zipper and some skin (a helper session's research, `docs/kh1-tint.md`). `tint.rs` now
  swaps the source textures of mesh-table entries 0 and 1 for recoloured copies (red → hue 225°,
  the rest untouched) made with KH1's own texture creator, in the form only. The PKG's remastered
  assets are readable now (`egs::Archive::read_remastered`, `Kh1Install::read_remastered`).

### 2026-10-18: Stage 196 (every KH2 particle program the spells use)
- "wisdom form's fire is lackluster, add all the KH2 vfx": a helper session audited every spell
  package the mod plays (Fire / Blizzard / Thunder / Cure / Reflect, Wisdom's and Sora's) and
  implemented the 20 program types kh2-fx skipped (`docs/kh2-effects.md` §7.5..7.10): the shape
  trails (155 / 157, `kh2-fx/src/tail.rs`: Fire's flame tails, Blizzard's sparkles, Reflect's
  streaks), the loop integrators (210..215), random vectors (63 / 75 / 76), the camera-facing
  and parent matrices, two model draws and vertex births. KH2's heat haze (160, 238) runs but
  draws nothing (it needs the screen behind it). Test `spell_packages_fully_supported` (with
  `KH2_FX_DIR`). The hook now passes the frame's camera before the effects tick
  (`Effects::set_camera`).

### 2026-10-18: Stage 197 (one direction per combo; forms run on MP)
- "sora can sometimes change the direction of his velocity depending on where you hold the
  stick regardless of if hes locked onto an entity or not" / "we do need to change how sora is
  able to change directions when attacking when he isn't locked onto anything", then the rule
  (the user): the combo stays locked in one direction until it ends (idle or a fall); with the
  orange or blue reticle on, the next attack snaps toward the enemy at its start, never mid
  attack. Each swing used to face the stick when nothing was targeted, and the motions' turn
  windows homed mid-swing. `Character::set_combo_direction_lock` (with `set_face_lock`), set by
  the hook. Test `combo_keeps_its_direction`.
- "drive forms consume mp over time, if mp reaches zero it reverts back instead of the current
  timer": 1 MP every 3 s of battle, 6 s out of it; at 0 MP, Revert. Entering needs more than
  the 3 MP it costs.

### 2026-10-18: Stage 198 (no spell held through a finisher; idle ends the combo)
- "during a finisher if a magic spell is inputted, then at the end of the finisher regardless of
  how much time is passed sora will always execute said spell ... there shouldn't be any buffer
  system like that for magic": a spell chosen in a move with no window left for it
  (`Character::magic_later`: a finisher without a next finisher) is refused at once (KH1's
  spells in `combat::command`, Reflect and Wisdom's spells in their waits); one chosen in a swing
  still goes at that swing's combo window.
- "if sora inputs a magic and returns to the idle state his position in the combo string is
  kept ... the intended behavior is for sora to have his combo reset if he returns to idle":
  `COMBO_GRACE` 60 → 2 frames; a KH2 cast played to its end on the ground resets the combo; a KH1
  spell's combo slot is restored at hand-back only when an Attack or a spell cut it or waited
  through it (`takeover::SPELL_CUT`).

### 2026-10-18: Stage 199 (turning mid-swing only toward a reticle)
- "have sora change the direction he's facing mid move only if the condition of an entity
  having a blue or orange reticle on it is met": with the combo's direction locked, the swings'
  turn windows run again when a target is locked (KH1's reticle, either kind, is the lock), and
  stay off without one. Test `combo_keeps_its_direction` extended.

### 2026-10-18: Stage 200 (forms drain yellow MP)
- "for drive forms, instead draining one bar of blue mp we have it drain yellow mp incrementally,
  when the value that stores yellow mp reaches zero then we take away one bar of blue mp": that
  is exactly KH1's payment of a cost under 100 (`docs/kh1-drive.md`): the form pays 1 every 2
  frames of battle (4 out of it); out of MP (KH1's own test fails for 1), Revert.

### 2026-10-18: Stage 201 (hair, hood and keychain move as KH2 animates them)
- "add the animations for soras hair and chain on each kh2 animation instead [of the physics],
  don't affect the KH1 animations": a helper session matched KH1's 14 hair spikes to KH2's 11
  (nearest roots on scaled heads, blended with the second nearest), the hood (KH1 80/81 ← KH2
  72, 82/83 ← 73) and the keychain (KH1 283..291 ← KH2 weapon 3..11, same places in the two
  binds), and `kh1-data/src/secondary.rs` turns them as KH2's motion turns its own (hair and
  hood relative to the head and chest, the chain link by link from KH1's pommel), lengths kept,
  guarded against the head and spine (`docs/kh1-secondary-motion.md`). The KH2 pose the hook
  keeps now carries the Keyblade's bones too. The physics swing runs only for chains KH2 didn't
  pose (the dream weapons and wooden sword). Ini: `physics = off` (all off), `kh2_hair = off`
  (the old swing). KH1's own animations never reach this.

### 2026-10-18: Stage 202 (the user's list: camera, Wisdom, guard)
- Wisdom: "finishing magic ... unable to be in the 2nd finisher slot when having a finishing
  plus": Wisdom's finisher (154) and finishing casts have no finisher range, so nothing could
  follow them. A finisher motion with none now takes the next finisher (swing or spell) in its
  last 6 frames (`finisher_window`). Test `second_finisher_after_a_finisher_without_a_window`.
- "when inputting a 2nd finisher, if you input magic then hit the attack command the attack will
  overide the magic": while a chosen spell waits for its window (KH1's queue, Reflect's,
  Wisdom's), an Attack press doesn't take the window from it (`takeover`).
- "sora should not loose his velocity when doing an air move in wisdom form", "his air attack
  ... freely able to move", "landing animation should preserve his momentum": a moving air
  attack (an allow-movement range) keeps his speed at its start; moving casts keep speed and
  fall and are steered in the air as a jump; with ground inertia the landing slides on instead
  of KH2's 14-frame hold. Test `wisdom_keeps_momentum_in_the_air_and_on_landing`.
- "animations that have new magic do not have their proper elemental hitbox and currently do
  physical damage": Wisdom's Fire / Blizzard / Thunder now land with KH1's own attack data for
  the spell at its level (element, max-MP source, reactions; a helper session's research,
  `docs/kh1-attack-elements.md`); the shots stay physical (KH1 has no known light element).
- "any kh1 magic that doesn't have a kh2 equivalent (gravity, stop) should use the normal
  thunder/cure wisdom form animation but still cast their respected magic": KH1 casts them; in
  the form its slots are drawn with the form's motions, the cast as Thunder's (Gravity) or
  Cure's (Stop, Aero), the air ones in the air (`pose::follow`).
- "Guard currently does not change soras direction to face an entity with a yellow or blue
  reticle": the guard starts facing the locked target. Test `guard_faces_the_locked_target`.
- KH2 camera: "any time that a move or action calls for the camera to be reset (limits,
  inspecting an object) the game reverts the camera back to kh1s camera ... it should call the
  default position of the kh2 camera instead": every KH1 camera reset ends with its follow
  camera starting over from hold (mode 0; event holds and "camera behind Sora", the end of a
  Limit's or summon's cinematic, room loads). A helper session made the KH2 camera catch that
  and start again at its own default behind Sora (facing the target when locked), keeping the
  view through the rest of a scene unless KH1 walks Sora itself (`docs/kh1-camera.md` 9.1, 9.2).
  Room entries now start behind Sora.

### 2026-10-18: Stage 203 (the Magic menu takes a spell only in KH2's windows)
- The user, on Attack taking a chosen spell's window: "in kh2 the game won't let you select a
  magic until the game lets you, keeping you in the menu until you back out with the back
  button". KH1's menu entry run (`0x1402912F0`, hooked in `combat::run_entry`) now returns
  without writing the command record when the entry is a spell (from the Magic list, a party
  target list or a shortcut) and KH2's move can't be cut by magic this frame (`can_cast`):
  no sound, the menu stays where it is, and the next press tries again. So a spell is never
  chosen to wait for a window, and an Attack pressed meanwhile is simply the combo's next
  swing. Drive forms and KH1's own casts keep their paths.

### 2026-10-18: Stage 204 (Drive and Revert on end frames, no attack buffer, air casts hold)
- Drive (the user's list): "you can't transform in the air ... sora inputs a drive in the air
  and should stay frozen in that position": the Drive is taken in the air and Sora is held where
  he is through it (`Character::play_held`: a scripted motion with no gravity and no drift),
  the arrival too, then he falls. "transforming ... should only take 45 frames": Sora's Drive
  motion runs 160 frames (80 at 30 fps); the swap now comes at 39, plus the form's 6-frame
  arrival (53), 45 in all.
- Revert: base Sora's motion 4, which the revert played, is his fall loop (A004), hence "stuck
  in the kh2 free fall animation". The revert now plays nothing: what was playing is cut and he
  is free with his movement carried over (`Character::carry_movement`: speed, fall or rise, in
  the air or not), so on the ground the stick's walk/run/idle takes over without halting him and
  in the air he falls. It works in the air too. Out of MP it happens at the first frame he can.
- Page 2's Drive row reads "Revert" in the form and reverts directly (no list). Drive and
  Revert are taken only on a frame Sora can change (`Character::can_change_form`: free,
  jumping, falling, landing, or an attack / cast in its combo, finisher or reaction-command
  window or last 6 frames): before that the press does nothing (`combat::run_entry`), as spells
  since s203.
- Attack (the user: "there is a buffer system ... We don't want a buffer type system to be in
  the game and only inputs to be accepted when its in a acceptable window"): `character-core`'s
  `buffered_attack` is gone; a press counts only on a frame the combo / finisher / cast window is
  open. An Attack pressed during one of KH1's moves (its spell's cast, a reaction) counts only on
  the frame it is pressed: if it can't cut the move then, it's dropped. Tests
  `an_early_attack_press_is_not_kept`, `drive_in_the_air_holds_and_swaps_carry_movement`.
- Air casts (the user: Reflect in the air "doesn't halt his gravity for a second ... like other
  magic moves do", Wisdom's too): the no-gravity frames of a cast motion hold him again (s174
  had dropped them for air casts so they fell as KH1's do); `air_magic_falls` now only ends a
  cast that reaches the ground in the landing.

### 2026-10-18: Stage 205 (footsteps follow KH2)
- "footsteps currently follow whatever the kh1 animations are doing ... we need footstep sfxs to
  follow what kh2 animations are doing": stage 155 gated `0x140293C70`, which is objects' (crates,
  pushables); Sora's steps come from his KH1 motion's sound events (`0x14029DA4E` ->
  `0x140293290` -> table `0x140529788` -> `0x1402939B0 (actor, kind)`, Steam `0x140295B40`):
  kinds 0..7 the feet (alternated by `+0x39C`), 10 a landing, others walls and objects. Hooked:
  while KH2 has Sora, KH1's are dropped.
- KH2's footstep is frame trigger 2 (param: kind, KH1's own numbering: run 0, walk 4, landing
  10). `kh2-data` loads them per motion and slot (`Rig::footsteps`, battle and field), the clip
  events carry them (`EventKind::Footstep`); the hook plays KH1's floor sound through
  `0x1402939B0` when the rig's walk / run track passes one, and for an action's events (the
  landing).
- Wisdom Form: none but the landing's ("we generally want footsteps disabled for sora in this
  form (with the exception of landing) as he is gliding"; its walk and run have no trigger 2).

### 2026-10-19: Stage 206 (air Reflect's sink, Reflect's guard, Drive revert, Quick Run landing, Wisdom's aura and KH1 spells)
- "revert our changes to base reflect and wisdom reflect, we do want it to fall like kh1" (then,
  asked: it "falls too fast"): s204's hold in the cast's no-gravity frames is undone; the cast
  stops his fall and he sinks at a share of gravity while it plays (`Character::
  air_magic_gravity`, ini `air_cast_fall`, default 0.3, estimated), the fall after it going on
  from the speed reached. Tests `air_cast_sinks_at_its_share_of_gravity`,
  `falling_air_cast_lands_into_the_landing` (now falls).
- "Reflect does not have a block hitbox": the barrier (ATKP 573's shapes) meets the enemies' attack
  objects each frame (`Strikes::barrier_meet`): a free-standing shot it touches gets KH1's clash,
  its half (`0x1402B2A30 (shot, barrier)`, Steam `0x2B4BC0`, the barrier a priority-100 guard
  object): turned back, counted for the counter; an enemy's melee attack gets KH1's clash record
  on its owner (kind 6 / 7, its script's recoil), with KH1's spark. Not tested in game.
- Drive: a form or Revert chosen, KH1's confirm bits are taken out of its battle button state
  until confirm is let go (the user: holding it after a revert opened the Drive list again). The
  revert starts Sora's package elements 15..27 (the Drive motion's 1..13 mirrored; 15 has sound
  115 as 1 has the change's 114; Data, KH2's call not traced), under `form::REVERT_FX`.
- Wisdom's arrival: its light on his body (elements 1..12, the form's colour) stays 90 frames
  whatever he does; its element 100 (the looping aura: sparkles at his feet, trails round him)
  stays for the whole form (s192 had ended both with the 6-frame arrival motion).
- Quick Run ends in KH2's Land (an air slide; the user: no landing after it); landing in a Quick
  Run, an Aerial Dodge or a jump's take-off goes to Land too. Test `quick_run_ends_in_the_landing`.
- KH1-only spells in a form (Gravity, Stop, Aero; drawn with the form's motions since s202): on the
  form's Thunder cast motion's clock (MAGC at the spell's level): Attack / a spell / jump / Square
  cut it only from that motion's reaction window, and it ends at that motion's length once the
  spell is out; on the ground he moves as the form's casts do (the stick in the motion's
  allow-movement range with the form's inertia, braking after), through KH1's mover
  (`takeover::FormCast`).
- KH1's leap off a ladder or a pole (state 16) is handed to KH2's fall once it drops (the user: KH1's
  fall; the log: states 15, 16, then 2 only ~35 above the floor).
- "sora is able to cancel his taking damage animations into attack": a damage reaction takes no
  Attack before it ends (a jump or the action button still cut it from its reaction window).
  Test `sora_flinches_and_is_pushed_back_by_a_small_hit` updated.
- Open: Wisdom's elemental hits still showed the physical spark in s205 (s202's spell data: its
  `+0x28` hit effect is looked up in Sora's effect group, `docs/kh1-attack-elements.md` §4).

### 2026-10-19: Stage 207 (the user's list: ledges, KH1's air states, lock-on, Wisdom's shoes, the Keyblade's vanish)
- "When sora lets go of a ledge he instantly regrabs the ledge": after KH1 lets go of him in the air
  (a ledge, a ladder, a pole) KH1's grabs aren't tried for 20 frames of KH2's fall (`NO_GRAB`), as
  KH1 skips them for 15 frames after jumping off something.
- "KH2 soras falling animation is unable to grab onto ladders/vines": the grabs are tried out of an
  Aerial Dodge or a glide too, and with the stick let go the probe move presses along his drift (or
  facing), so KH1's climb check has a wall face to read.
- KH1's jump or fall with him free (state 1 / 2, KH1's can-act test passing, not a spell's air
  cast): KH2 takes him at once (two frames in), rising or falling (`Character::carry_into_air` now
  carries a rise), its Aerial Dodge ready (`allow_air_dodge`); KH1's jump and Square are KH2's
  meanwhile, so KH1's glide and Superglide don't come out of it (the user's fail-safe). All through
  one of KH1's commands (a spell, an item) KH1's jump and Square bits are KH2's too: on the frame it
  lets him act again KH1 read them as its own jump, high jump or dodge roll (the user).
- Lock-on: locked onto a target, the combo rows are measured against that target only (the user:
  one ahead locked, another beside him: the base finisher, lunging at the one beside him).
- Wisdom's blue takes in the shoes: mesh-table entry 3 (`-1c3`, shoes and the yellow pouches), its
  yellow (hue 48° ± 16) to blue (`tint::FROM`). The pouches go blue with them (same texture).
- The Keyblade's vanish: as KH2 puts it away, element 0 of the Keyblade's own package
  (`obj/W_EX010.a.us`: a flash that swells and fades over 13 ticks with a turning glint) starts on
  the blade (half way from the hand to the tip). Inferred: KH2's own call for it isn't traced.
- Open: after a boss falls, KH1 holding Sora with its own animations before the scene (needs a log).

### 2026-10-19: Stage 208 (the user's lists: KH1 takeover frames, walking, Wisdom's KH1 spells, Reflect, hits)
- Built on s207 (claude-6d's s206/s207 from the GitHub line, applied onto s205).
- Walking: the stick's direction is now KH1's move vector (`0x142D5F630`, written by its think
  `0x1402A4090` -> `0x14028A650`: the stick through the camera), not Sora's `+0x20`, which
  KH1's walk turns toward the stick slowly ("only keeps him moving at the direction hes facing
  prior"). The same for the form casts of KH1 spells (s206) ("sora does not change his movement
  direction to the analogs sticks direction"). A log line when the two differ.
- Wisdom's KH1-only spells: the form's cast clock ends at its motion's length (Thunder 62: 25
  frames at 30 fps = 50) whether or not a shot was counted ("kh1 stays in the animation for
  another 17 frames"), on frame 50; his slide starts from KH2's speed going in and is handed to
  KH2 at the end (`Character::carry_on_ground`; "we don't want sora to have his momentum
  halted").
- KH1 takeover frames (`takeover::kh1_cut_frame`): grounded KH1 Fire from frame 49, taken only
  with an Attack or a Drive chosen then ("only if conditions are met"); an item from frame 51,
  taken whatever is pressed ("on frame 52 ... they will all be kh1s animations"). Items are told
  by the list they were picked from (`combat::run_entry`: KH1's Items list `0x140529670` or a
  party list under it). A Drive chosen during KH1's move goes through `take_chosen_command`
  to `form::choose` (`accept_now` / `kh1_cut_window`), KH2 takes him (`DRIVE_CUT`) and changes.
- Air KH1 spells: KH2 lands him on the frame he would touch down (`touchdown`: falling with
  less than 1.5 frames' drop to the floor, put down through KH1's mover), not a frame after
  KH1's landing (its roll or jump could come out there).
- Air Reflect (base and Wisdom): Final Form's physics (`Character::hold_air_cast`, P_EX100_TRIF
  D373: stopped, no gravity frames 0..54, no drift, then a plain fall), their own animations.
  Test `air_reflect_holds_as_final_forms`.
- Revert: the command menu's cursor back on Attack (root `+0xC`).
- Wisdom Fire / Blizzard / Thunder hits: the attack's effect object (`+8`) is KH1's magic
  effects (`0x142EFBE90`, as KH1's own spell handlers `0x14039FD60` and others set it), not the
  Keyblade's (the row's attack takes the weapon's, `0x1402B10E0`), so the hit effect
  (`0x1402B2190`) is the spells' (the user: "still the physical keyblade effect"). Unconfirmed.

### 2026-10-19: Stage 209 (the user's s208 report)
- Walking: under KH1's own dead zone (a slow walk, KH1 still shows idle) its move vector is
  empty, and the fallback was his stale `+0x20`. Now the raw stick turned through KH1's camera
  as its think does (`0x14028A650`: rotate by minus `0x14299C504`, mirror x, rotate by
  `0x14299CB94`; not in camera modes 2 / 0xB), its 8 sign conventions scored against KH1's own
  vector on frames that have one (`takeover::stick_through_camera`). KH1 refreshes its vector only
  while Sora can act, so during its casts it was stale too (Wisdom's KH1-spell slide kept the old
  direction): it counts only then.
- Wisdom's KH1 spells: the form's cast clock ends the air ones too (KH2's fall from there).
- KH1's menu takes nothing while Sora can't act (`0x1402912F0`, its `0x1402A0D30` test), which is
  why Attack only came out at Fire's end and the Drive list was greyed: in KH1's cut window
  (`kh1_cut_window`) `combat::run_entry` takes Attack (`ATTACK_AFTER`) and a Drive form
  (`drive::chosen`, the menu chain back to its top) itself; `form::usable` counts the window.
- Items: command type 3 (the log's "command 0x3 (02000803)"), not the list they came from (the
  item end frame never fired).
- KH2's first frames after KH1's move don't carry KH1's push speed (`+0x120`, KH1's own walk /
  run speed then, 708/s in the log): the teleport forward at the next attack. Those frames are
  logged.
- Undone: s208's spell hit effect object. `0x142EFBE90` is the current object of KH1's script
  command dispatcher (`0x1403975E0` stores it before each handler), not a lasting magic effect
  set; KH1's element hit sparks belong to its spell objects, which exist only during its own
  casts. Still open.

### 2026-10-19: Stage 210 (KH1's leftover motion after a cut)
- The user: past frame 50 of a form's KH1 spell, KH1 "still thinks its in the aero animation
  (0x3A)" until the stick moves, and an Attack waits 21-25 frames. The s209 log: the cut handed
  him to KH2, and the next frame "KH1's own action, slot 55" took him back (KH1's cast motion
  plays on after our cut). Now a cut (`cancel`, or the air cast's landing) sets `CUT_LEFT`:
  until KH1's slot is a plain one (0..7) or KH1 takes him for something new, the leftover isn't
  KH1's own action. KH1 greys its menu while its cast plays, so Attack is taken in
  `combat::run_entry` as KH2's press meanwhile (`takeover::cut_left`).

### 2026-10-19: Stage 211 (Valor Form)
- The user: "i want to try to add valor form, and add the slot in menu for valor form, and
  automatically put a keyblade that isnt equipped into the slot. If no keyblade is eligible to be
  in that slot, dont let players go into valor form".
- `form.rs` takes several forms (`form::Kind`: Valor, Wisdom, in KH2's order; `drive::FORMS`
  gets Valor first, record 0x7F, word 0x1FD; Wisdom keeps 0x7E / 0x1FE, its argument now 0x11).
  Valor: `P_EX100_BTLF` (moveset 2: its swings with both Keyblades' hitboxes, the rig's two
  weapons), High Jump MAX (as Wisdom's Quick Run), no magic (spells refused with KH1's "can't"),
  its motions' effect elements not played (they're its own archive's, not loaded yet), Sora's
  footsteps. Wisdom-only paths (`formmagic`, the KH1-spell casts on Wisdom's motions, the tint,
  the footsteps rule) test `form::in_wisdom`.
- Valor's Keyblade (`valor.rs`, `equip.rs` from a helper session: `docs/kh1-equipment.md`): a
  Keyblade Sora owns with a copy not equipped; the save's `+0x488` keeps it while eligible, else
  the one with the most Strength; none: Valor greyed and refused.
- Stage 211b: Valor's effects: its archive (`P_EX100_BTLF.a.us`, PAX "p_ex") is effect package
  `kh2fx::VALOR` (after Wisdom's spells'); its motions' frame trigger 1 starts those elements on
  Sora (`kh2fx::form_event`), the ones flagged to end with their motion ended when it changes.
  No element sounds for them yet.

### 2026-10-19: Stage 212 (Valor's row on the Equipment page; Valor's effects)
- A helper session added `equipmenu.rs` (`docs/kh1-equipment.md`, "Implemented"): Sora's slots
  window gets a "V: <Keyblade>" row under his weapon row (rows + 1 through a wrapper on the
  window's update pointer, a shadow of his record behind `r12`), confirm on it opens KH1's
  weapon list filtered to the free Keyblades, the pick stored at `save + 0x488` with Sora's own
  weapon put back; help line and accessory rows mapped one down. All "Equipment:" in the log.
- Valor's effects (stage 211b).

### 2026-10-19: Stage 213 (Valor's left-hand Keyblade)
- A helper session added `leftblade.rs` (`docs/kh1-weapon-second.md`, its section 7 the checks):
  in Valor the Valor Keyblade's `.wpn` is loaded with KH1's loader into our own buffer (KH1's
  `.wpn` release handler `0x1404DAF48` wrapped to skip it), set up as a model, and drawn by a
  second call of the model queue `0x1401D1F10` with Sora's body meshes hidden and the left-hand
  matrices in bones 280..292 for that call only (from KH2's own left Keyblade, the rig's second
  weapon, calibrated on the right hand). ini `valor_left_blade` (default on; off = nothing
  installed). All "left Keyblade:" in the log.

### 2026-10-19: Stage 214 (Valor: Keyblades kept out, Upper Slash)
- "dont let soras keyblades disappear in valor form": in a Drive Form the rig is always in its
  battle stance (`anim.field` false), so the Keyblade isn't put away out of battle (KH2's forms
  hold theirs); the left one (s213) draws only while the right one is out.
- "make sure that soras square launch attack works in valor": KH2's Valor table has none of
  Sora's Square moves (its kind-0 rows are 11, 12, 38, 40: guard and finishers). Sora's Upper
  Slash (row 37, motion 167, ability 95) is copied into Valor's data at load
  (`pose::port_moves`: the row, the clip, the body's and the right Keyblade's animation), usable
  while Sora has Upper Slash equipped.

### 2026-10-19: Stage 215 (sounds: Reflect for everyone, Valor's own)
- "the reflect sound FX arent working for some people": Reflect's (and the forms') sounds were
  loaded only with `kh2_sounds` on, and one unreadable file (Sora's `P_EX100.a.us`, a sound bank
  that didn't parse) dropped every KH2 sound. Now Reflect's and the forms' load whatever
  `kh2_sounds` says (it decides Sora's moves only), and a missing package or bank costs only its
  own sounds (logged).
- "make sure also that valor form has its proper sound FX": Valor's effect elements' sounds (its
  archive's PAX, package `kh2fx::VALOR`, played as they start), its Keyblades' swing sounds and
  voices (its clips' frame triggers 8 / 14) load and play in the form.

### 2026-10-19: Stage 216 (Valor's Square, the left Keyblade's load)
- "using sora's square move in valor doesnt launch him still, he also should be able to use it
  while walking": Valor's guard row (11) is an attack (motion 181, a rising slash with both
  Keyblades, jump 90); the log showed it run as a guard. A guard row whose clip has hitboxes and
  no guard window is now an attack: Square on the ground, standing or moving, starts it as a
  forced row (its launch from the row's jump). Test `guard_row_attack_launches_from_square`.
- "the second keyblade isnt there": the log, "item 0x5b's model not used: header 0x80 0xa800
  0x1f700 (size 0x1f690)": the `.wpn`'s `+0xC` runs past the bytes loaded (end padding), and
  `leftblade::set_up` refused it. Only `+4` / `+8` are checked now, as KH1's own handler reads.

### 2026-10-19: Stage 217 (Valor's Keyblade swap in the form, sound diagnostics)
- "when switching the second keyblade out while in valor form it doesnt swap properly": the
  left Keyblade was loaded only at the form's start. While he is Valor the slot is read every 10
  frames (`form::after_step`) and `leftblade::show` gets it each time (no-op when it's loaded;
  asks again while another load is under way).
- "the sounds are still not there": the s216 log has Valor's sounds loaded (package 17's
  elements, its swings 5000xx) and the volume offsets check out against the Steam exe (KH1's own
  reads of `0x14012AC50`'s settings, all 4 bytes later). `kh2sound` now logs its first 40 plays
  ("KH2 sound played: ..."), sounds asked for but not loaded, a muted volume, and Windows
  refusing the sound device, which `play` dropped silently.
- The left Keyblade's trail: a helper session is on it.

### 2026-10-19: Stage 218 (the left Keyblade's trail)
- "the secondary keyblade doesnt have its trail": a helper session gave it KH1's own swing trail
  (`leftblade.rs`, `docs/kh1-weapon-second.md` section 8): the trail id KH1 is asked for Sora
  (`0x1402C0160`, hooked) on an owner of ours (a copy of Sora's actor and instance with the
  left-hand weapon bones), made as the left weapon's own trail windows open (read from
  `W_EX010_BTLF_L`), faded as they close. "left Keyblade: trail ..." in the log.

### 2026-10-19: Stage 219 (KH1's motion after its air spell's touchdown)
- The user's videos (a memory watch on Sora's `+0x164` motion request, `+0x168`, `+0x16C`
  time): landing from a KH1 air spell, KH2 showed its jump pose, KH1's motion stayed the cast's
  (0x84, air Blizzard) for 6 frames, then KH1's own landing (0x07) blended in. Two causes:
  `AIR_START` read KH1's height above the floor (`+0x3D4`) right after we put him on it, still
  its last frame's ~40, so KH2 started him in the air; and KH1's cast motion went on with its
  own logic. Now the touchdown doesn't start KH2 in the air, and on every cut of a KH1 move
  (`cancel`, the air cast's landing) KH1's motion is set to its idle (`0x14029D7F0 (actor, 0,
  0)`), logged.

### 2026-10-19: Stage 220 (Valor's second Keyblade: trail, menu, stats; forms' MP; Sephiroth)
- The left trail followed the right blade: KH1 takes a point along a bone from its parent's
  world matrix and the bone's local offset (`0x1401D41C0`), and the grip's parent is the right
  hand. The trail owner's matrices are now all of Sora's moved by the right-grip-to-left-grip
  turn. Its look is the left Keyblade's own: the `.wpn`'s effect section (`+4..+8`) is set up as
  KH1's handler does (`0x1401DDF20`, released with `0x1401DED70`) and used for the trail (the
  right one's when it has no such trail).
- Equipment: KH1's live preview runs during the Valor pick (the hovered Keyblade shown, put back
  by KH1's own restore on close).
- Valor adds its Keyblade's Strength and max MP to Sora's each step (from KH1's own figures,
  `0x1402A2FA0`), taken off when it ends (`valorstats.rs`).
- Valor's fall and landing clips are base Sora's no-Keyblade ones (identical in both msets): its
  idle stands in for the landing (`RigAnim::stand_in`).
- Forms drain one unit every 3 frames of battle (was 2), and each melee hit in a form charges MP
  with KH1's own charge (`0x1402A2950`): the hit's units (attack data `+0x2C`) × his bars, as a
  hit is to a point outside a form.
- Sephiroth's pillar killed through Leaf Bracer / Second Chance: a lasting KH1 attack re-arms its
  hit on an interval and KH2's reaction kept Sora in it. No hit lands while he is knocked down,
  nor the same attacker's within 20 frames while he flinches (`hurt.rs`, `docs/kh1-rehit.md`).

### 2026-10-19: Stage 221 (an air spell's touchdown, again)
- The user: s219's landing fix changed nothing. KH1's set-motion (`0x14029D7F0`) only queues a
  request (`+0x178` pending, `+0x17C` the motion); our state change to the ground one ran after
  it, and leaving KH1's air state queues its own landing (0x07) in its place. Now the state is
  set first and KH1's idle queued last. And the touchdown put him on the floor while KH1's height
  above it (`+0x3D4`) still read its last frame's ~40, so KH2's first step started him in the
  air (its jump pose): set to 0 there. KH1's motion fields are logged for 12 steps after a cut.

### 2026-10-19: Stage 222 (the left trail's place)
- The user's video (s221): the left trail, in the Valor Keyblade's own look, was drawn well off
  the blade. Its owner's matrices were made from our pose (KH1 model space × our world) at draw
  time; now each KH2 step from Sora's own world matrices as KH1 last drew him (what KH1's right
  trail reads), each moved by the right-grip-to-left-grip turn of the same frame. A live left
  trail's position, the left grip and its last bone are logged for its first steps.
- KH1's air spells back to s207's handling (the user: "revert the changes made to aerial magic to
  before s207 ... its completely fucked atm"): KH2 takes him when KH1 has landed him (state 0 on
  the floor), with KH2's landing; out are s208's touchdown a frame early (he was moved onto the
  floor), s209's form casts ended in the air, s219's / s221's KH1 idle request, cut flag and
  height reset on the landing. KH2's air Reflect (Final Form's hold) stays.
- Valor refuses KH1's specials (Sonic Blade, Strike Raid, ...): every command of Sora's KH1 runs
  as a move other than Attack, magic, items and its two air-allowed types, logged.

### 2026-10-19: Stage 223 (the left trail's turn)
- s222's log: the left trail's effect sat on the left grip (a frame behind), yet the video showed
  its ribbon well off the blade in a steady direction. KH1 makes an effect's angles from its
  matrix's rows, normalized (`0x1402A6D60`: atan2 / asin), so a mirrored or sheared grip matrix
  reads as a wrong turn. The owner is now a rigid copy of the right hand's Keyblade (every bone
  × one turn) whose turn is the left grip's made proper (orthonormal, the right grip's scale;
  mirrored, the axis least along the blade turned round), filled at draw time right after the
  left Keyblade is drawn. Both grips' determinants are logged.
- Choosing Valor's Keyblade, KH1's preview puts the hovered one in Sora's weapon byte, so the
  stats shown were his with that Keyblade alone (the user: Strength "capped at 70"): his own
  weapon's Strength and MP are added then (`equipmenu::valor_preview`, `valorstats::add`).
- Wall clinks on again by default (`wall_clink`, the user); both inis set on.

### 2026-10-19: Stage 224 (forms' MP from hits taken out)
- The user: KH1's own MP charge on a hit already reached Sora in forms ("at level 100 it was hard
  to notice"); s220's extra charge per melee hit in a form is taken out. The slower drain stays.

### 2026-10-19: Stage 225 (air Reflect falls as KH1's spells again)
- The user: "revert the changes to reflects physics so that it falls like kh1 magic ... make sure
  it keeps the other changes to reflect just the physics changes". s208's Final Form hold
  (`hold_air_cast`, 54 frames without gravity) is no longer asked for: the air cast sinks at
  `air_cast_fall`'s share of gravity (s206) and lands into KH2's landing (s177). Everything else
  of Reflect's (barrier, guard, counter, sounds, menu, combo) is as it was.

### 2026-10-19: Stage 226 (the user's Reflect / Explosion / air magic list; dream weapons)
- Air casts (Reflect): his fall stops as the cast starts, then full gravity from rest
  (`air_cast_fall` default 1.0; test `air_cast_stops_the_fall_one_frame_then_falls`).
- No spell in KH2's landing (`cast_window`: `Land` no longer takes one; test
  `no_cast_during_the_landing`).
- Reflect (helper, `refl-fix`): one counter at the barrier's end whatever Sora does, all hits
  counted; melee attacks meeting the barrier are counted, not clashed (that recoil looked like a
  counter per hit); its element 11 plays where each attack meets the barrier (sound 139, element
  11's own 174 is empty on PC). Sora is untouchable as in KH1's dodge roll (actor flag 0x40, which
  the hit pass skips: no record, no hit effect, no grab) through KH2's invulnerable frames (Dodge
  Roll, Quick Run, Aerial Dodge, Aerial Recovery) and Reflect (`docs/kh1-intangible.md`).
- Explosion's last blast (ATKP 912: up 15, sideways 30) launches as KH2's default blow's height
  and twice its distance (`combat::FARTHER`).
- KH1's air spells: KH2 lands him on KH1's ground state alone (its height above the floor could
  still be the last frame's, and that frame went to KH1's landing and its jump), only once KH1
  had him in the air for it; KH1 doesn't see jump or Square through the air cast (`probe::pad`).
  KH2's sideways speed is carried into KH1's air state (`+0x3C0`) as the air command starts.
- Dream weapons and the wooden sword (`0x52..=0x55`; a tester: the Dream Sword stretched as a
  movement began, the Dream Shield held otherwise in battle than in cutscenes): every weapon bone
  held rigidly as KH1 holds it from his hand (`pose::dream_hold`, noted from KH1's own pose), also
  through the blend back to KH1's animation.

### 2026-10-19: Stage 227 (riding up a target in the air; Explosion higher)
- The user: locked on in the air, attack after attack rode up the enemy's body until Sora was
  above it and his swings missed. A rising frame took the height KH1's mover left him at, and
  pushed out of an actor's body it lifted him; now the mover may cut a rise short (a ceiling),
  never add to it (logged).
- Explosion's last blast launches a third higher than KH2's default blow (up 32, sideways 60).

### 2026-10-19: Stage 228 (air Reflect: a brief hover, then the fall)
- The user, asked which of the falls tried was closest to KH1's air magic: "brief hover, then
  fall" (s204's). Air casts hold him through their clips' no-gravity frames again, then fall
  (`air_cast_fall`'s share of gravity, default full); a cast reaching the ground still goes into
  KH2's landing. Tests `air_cast_hovers_then_falls`, `air_cast_sinks_at_its_share_of_gravity`
  (after the hold), `falling_air_cast_lands_into_the_landing`.

### 2026-10-19: Stage 229 (Reflect's physics back to s177's, one pop; KH1 footsteps; Valor's landing; Wisdom's KH1 spells cut; Aerial Dodge's sound)
- Air casts fall through their no-gravity frames from rest again (s177's; the user: "roll back
  the physics of reflect to version s177"), into KH2's landing.
- Reflect pops once per cast (helper, `refl-once`): the script's counter attack 574 stands
  frames 30..57 and re-arms every 10/8/6 frames, so it hit each foe 3..5 times before the burst;
  now only the burst (667/668/669, tier from all hits counted) strikes, once per foe at the
  cast's end; 574's effect still plays. Test `reflect_pops_once_for_two_hits`.
- Footsteps: KH1's own only (the step sound unhooked; KH2's footsteps from s205 off).
- Valor: KH2's landing plays again (s220's idle stand-in taken out).
- Wisdom: a KH1-only spell (Gravity, Stop, Aero) is cut by an Attack from its cast motion's input
  frame (Thunder's, `AllowReactionCommand`), whether or not KH1 counted its shot yet.
- Aerial Dodge (motion 202) in `kh2_sounds`'s default list: its sound 999019 at frame 1.

### 2026-10-19: Stage 230 (Drive heals; no specials in Valor; the left Keyblade steady)
- A Drive restores Sora's HP to full as it starts (`form::heal_full`, KH1's own HP change
  `0x1402A2790 (actor, amount)`, which keeps its HUD mirror).
- Valor: KH1's specials still came out (the user: Strike Raid, Ragnarok): the command step runs
  command type 99 as attack row 0x3E (`0x1402A4760`), and s222 had let 99 and 0x47 through. Now
  Valor refuses every command of Sora's KH1 runs as a move but Attack, magic and items, and its
  attack-row executor refuses any row for a command other than Attack (logged).
- The left Keyblade jittered as Valor fell in place: its bones were kept in KH1's world, so a pose
  a frame old was drawn where he had been. Now kept in model space and placed with the frame's
  own placement, and paired with KH2's left weapon of the very step whose body is drawn (a
  signature of the body's bones).

### 2026-10-19: Stage 231 (Final Form, a secret)
- The user: "lets work on implementing final form ... instead of it being listed in the drive
  menu, lets make it a secret": a button code on the pad (kept out of these notes and stored
  unreadably in `form::CODE`, the user; at most 1.5 s between presses; another watched button
  starts it over) asks for Final Form as the Drive list would (`form::pad`, read in
  `probe::pad`; taken in `form::before_step` when KH2 can change him: free, jumping, falling,
  landing or at a move's end, not in a form). The Drive list is unchanged.
- Final Form: `P_EX100_ULTF` (moveset 5, form id 5; TRIF is Master Form), the same change, cost,
  drain and revert as the others, its growth ability Glide at MAX, its own effect package
  (`kh2fx::FINAL`, after Valor's) and sounds, KH1's specials refused as in Valor.
- Its second Keyblade is always the Kingdom Key (item 0x51; the user: "just default it to
  kingdom key and keep the menu how it is"), with its Strength and MP as Valor's Keyblade's.
- Its Keyblades float where KH2's weapon motions put them (W_EX010_ULTF_R / _L on bones 178 / 86;
  their idle roots some 70 units from the hands, Valor's 5): KH1's weapon bones keep the turn from
  base Sora's held Keyblade root to KH1's grip, put on Final's root now
  (`leftblade::final_float`); the left one follows by the same turn (its reach check 400 there).

### 2026-10-19: Stage 232 (Final Form's code kept secret)
- The user: the code is removed from every readable part (notes, comments, logs); the table in
  `form.rs` is stored encoded. No change in play.

### 2026-10-19: Stage 233 (the user's 19:57 list; Final Form's combos)
- Footsteps: KH2's back (s229 undone).
- Air casts (Reflect) fall at KH1's own gravity after the stop: KH1's air state falls
  0.4847 × 0.5 × t² (`0x1402AD3A0`, its step `0x1423405DC` 1.0 at 60 fps) and at most 20 a frame;
  the controller's air cast takes a gravity share over 1 and its own fastest fall
  (`air_magic_fall_max`); `air_cast_fall` is now a share of KH1's.
- Final Form: a swing meeting a wall with no repelled motion in the moveset (Final has no 41)
  went back to its frame 0, met the wall again, for ever: "repeats the same moves ... soft locks
  you" (wall clinks on since s223). Now only pushed back (test
  `wall_hit_without_a_repelled_motion_doesnt_loop`). Its air combo ended after two hits: an air
  attack with an allow-movement window dropped its no-gravity frames (Wisdom's shots' rule); now
  only one moving all through does.
- Lock-on: an air swing at a target just below (within 150 past its radius) hops no higher than
  reaches it (test `air_swing_hops_less_at_a_target_below`).
- KH1 spells: Attack pressed during any of them is taken (KH1's menu refused it while it held
  him) and cuts the spell once it is out (a form's at its cast motion's input frame), the combo
  going on; pressed too early it is dropped.
- Clanks (helper, `clank`): KH2 swings clash along the drawn KH1 Keyblade (six points, KH1's own
  blade range per weapon, KH1's clash radius), swept between frames, against guards, walls and
  enemy attacks (KH1's attack-against-attack clash); Aerial Dodge carries a KH1 guard object (a
  parry). `docs/kh1-clash.md`.
- Reflect (helper, `refl-voice`): the pop at the barrier's end whatever the hits (the hold for
  hits taken out; the pop strikes with the counter at frame 30, the burst row's damage); a random
  KH2 attack grunt on each cast and voice 25 (its finisher line) on the finisher.
- Looks (helper, `form-colors`): Wisdom's sleeves blue; Valor's sleeves, belt, hood lining and
  shoes red (`tint::Look`, `docs/kh1-tint.md`).

### 2026-10-19: Stage 234 (the user's 21:25 list; command menu)
- Lock-on: s233's lower hop at a target below undone.
- Wisdom: an Attack pressed during a KH1 spell while frozen frames hold the cut is carried
  (`CARRY_MAX` 10 frames) and goes on into KH2's combo; Cure is KH1's Cure on KH2 Cure's motion
  (only Fire/Blizzard/Thunder are KH2's in Wisdom), falling and ending like the other KH1 spells.
- Forms get Aerial Recovery when the moveset has its clip; its guard and invulnerable frames come
  from the clip's windows (KH2's frames 0-16), not a fixed rule.
- Aerial Dodge: no invulnerability, only its Guard window (ATKP 406, a KH1 guard object).
- Guards (helper, `guards`): KH2 Guard windows are KH1 pure guard objects (clash type 4, priority
  100) for Aerial Dodge and Aerial Recovery.
- Valor: Square in a combo window goes to the finisher (rows 12 ground, 38 air,
  `square_finishers`); the left trail runs along the second Keyblade's own blade range; the form
  effects anchor on the Keyblades, sent in Sora's facing (helper, `valor-fx`).
- Final: air casts are KH2's own (no KH1 fall).
- Voices (helper, `kh1-voice`): KH1's swing grunt (group 4, random) on the dodge roll; KH1's
  "Give me strength!" (clip 37, inferred) on a Drive.
- Command menu (helper, `cmd-menu`): the Drive list's title reads DRIVES and its panels are white
  (list draw `0x140277F40` hooked); Items and Summons refuse a confirm Sora can't take and stay
  open, like Magic; Magic is greyed in Valor. `docs/kh1-drive.md`.

### 2026-10-19: Stage 235 (KH1's air spells land into KH2's landing)
- The user: "rewrite kh1s magic animations so that if they're used in the air they can call for
  kh2s landing animation". KH1's state change (`0x1402AF7D0`, Steam `0x2B1960`) is hooked: when
  KH1 itself puts Sora down out of an air spell (state 1 / 2 to 0 while it has him for the cast)
  inside his physics step, KH2 takes him at once, after that step on the same frame, into KH2's
  landing (`TOUCHDOWN`, a second `takeover` that frame). Before, KH2 took him a frame later, so
  KH1's landing frame showed and ran first. A landing KH1 makes outside the physics step is
  logged and left to the next frame's hand-off as before.

### 2026-10-19: Stage 236 (move styles: KH1 or KH2 per kind of move)
- The user: toggles for "base combo KH1 | KH2, aerial base combo KH1 | KH2, magic kh1 | kh2, aerial
  magic kh1 | kh2"; asked: in the Config menu and the ini, Gravity / Stop / Aero as KH1 spells on
  KH2 cast motions, base Sora only. `styles.rs` (ini `ground_combo`, `air_combo`, `magic`,
  `air_magic`; defaults combos KH2, magic KH1, as before). `docs/kh1-move-styles.md`.
- Combos at KH1 (helper, `style-combo`, `stylecombo.rs`): Attack runs KH1's own combo (its rows,
  finishers, abilities), KH1 keeps the presses, KH2 takes him back once KH1 lets him act; the
  style is picked by where he is when Attack is pressed.
- Magic at KH2 (helper, `style-magic`): Fire / Blizzard / Thunder / Cure are KH2's base-form spells
  (Wisdom's path, base Sora's MAGC cast motions, magic sub level × 2 − 2); Gravity / Stop / Aero
  KH1's on base Sora's KH2 cast motions (`BASE_KH1_CAST`); Aerial Magic at KH2 gives air casts
  (Reflect too) KH2's air physics.
- Config menu (helper, `style-menu`): KH1 doesn't scroll its list, so ours is a second page (rows
  8-12: Camera Style, then the four styles) drawn in KH1's rows' places while the cursor is past
  row 7; KH1's rows' panels (`0x1402D5590`) are skipped then. s224's 25 pitch is gone.

### 2026-10-19: Stage 237 (the style rows' names)
- The Config rows read as the user wrote them: "Base Combo", "Aerial Base Combo", "Magic",
  "Aerial Magic" (were "Ground Combo", "Aerial Combo"). The ini keys stay `ground_combo` /
  `air_combo`.

### 2026-10-19: Stage 238 (KH1's air spells call KH2's landing, from scratch)
- The user: "scrap everything related to aerial magic transitioning to kh2 landing animation and
  start fresh ... kh1s aerial magic animation states are hardcoded to transition to kh1's landing
  animation ... code [them] so that they are able to call kh2's animation states and call kh2's
  landing animation". Out: s235's state-change hook and its second takeover in a frame, and
  s207's rule (KH2 took him on the frame after KH1 had landed him, its landing queued).
- Where KH1 hard-wires it: Sora's player step `0x1402A75B0` (Steam `0x2A9740`; from the actor
  kinds' switch at `0x14029593C`), on a frame his state changed (flag `0x400`): state 2 to 0
  (old state `+0x378`) asks for KH1's landing, motion `(+0x3D0) + 7 | 0x1000000`, and
  `0x14029D8A0 (0x40000000)`; to state 2 its fall (6). Hooked: during KH1's air spell that landing
  is not asked for (the old state hidden from the step for the call) and `AIR_SPELL_LANDED` hands
  him to KH2 into its landing (`LAND_NOW`). If the step never saw it, KH1's ground state a frame
  later does the same, logged ("... without its landing seen").

### 2026-10-19: Stage 239 (move styles: Jump and Glide)
- The user: "a setting for kh1 jump or kh2 jump and ... kh1 or kh2 glide". `styles::Kind::Jump` /
  `Glide` (ini `jump_style` / `glide_style`, default kh2; `glide` still picks KH2's glide
  button), Config rows 13 / 14 on the second page.
- Jump at KH1 (helper, `style-air`, `stylejump.rs`): a fresh jump press while KH2 has him free on
  the ground runs KH1's ground jump handler (`0x1402CA3F0`: its jump or high jump), KH2's ground
  speed carried; KH1 keeps him through the jump, grabs and landing; KH2 takes him once KH1 lets
  him act for its own move (Attack with Aerial Base Combo KH2, a fresh Square, a jump press
  falling for KH2's jump-button glide, a command, waiting, then by Aerial Magic).
- Glide at KH1: KH2's glide off; a fresh jump press falling in KH2's air puts him in KH1's air
  state 2 and starts KH1's Superglide (`0x1402B05C0`) or Glide (`0x1402B0530`) by its abilities
  (`0x140282A70 (4 / 3, 0xB)`); KH2 takes him back into its fall or landing when it ends.

### 2026-10-19: Stage 240 (move style: Dodge Roll)
- The user: "add an option for switching between dodge roll". `styles::Kind::DodgeRoll` (ini
  `roll_style`, default kh2), Config row 15 (the second page now holds 8 rows).
- KH1 roll (helper, `style-roll`, `styleroll.rs`): KH1 rolls from its button handler
  `0x1402A59A0` (Square in ground state 0, not in deep water `0x1402BF530`, the Dodge Roll ability
  bit `0x20` of `0x142F5F610`; Guard `0x10`), turning him (`0x140295E20` / `0x140295C80`) and
  starting motion `0xDC` (`0x1402C7CB0`). At KH1: a fresh Square with the stick while KH2 has him
  free on the ground runs that, KH1 keeping him while `0xDC` plays; Square standing still stays
  KH2's guard; no ability or deep water, KH2's. Quick Run stays KH2's roll's. Steam map override
  `0x2F5F610:0x2F5EC10` (data).

### 2026-10-19: Stage 241 (air spells' KH2 landing off; jumps out of KH1's moves)
- The user: "comment out anything related to having kh1 magic animations transition into kh2
  landing animation ... causes more bugs then fixes anything". s238's player-step hook is no
  longer installed and the air spell's hand-off at landing is commented out (`cast_landed`
  false): KH1's air spell lands in KH1's landing, KH2 takes him after it as after KH1's other
  moves. The code stays for later.
- After a KH1 air attack (Aerial Base Combo KH1), the user: holding jump through the landing
  jumped on the first free frame ("buffer a jump, this is unintended"); a direction + jump in
  KH1's landing gave KH2's run, not a jump; KH2's high jump out of it went "way higher than it
  should". A jump out of a KH1 move is now only a fresh press made while KH1 has him on the
  ground (`GROUND_JUMP`; a press in the air, or the button held from it, doesn't count). It is
  KH2's jump on KH2's first frame that takes input, frozen frames between (`PENDING_JUMP`, up to
  30): KH2's own "held" memory was stale (the jump KH1's air attack came out of), so a press in
  the landing read as held. KH2's held state is set from the button at the hand-off
  (`JUMP_HELD_OVER`). For 40 frames after a KH1 move KH1's push doesn't lift him in the air (not
  on a platform), and those frames are logged ("after KH1's move: ...") for the high jump, whose
  cause isn't confirmed.

### 2026-10-19: Stage 242 (KH1 glide on Square, KH1 roll, KH2 glide back, Battle Quotes row)
- The user (KH1 glide): "when you input an aerial dodge glide takes priority can you rewrite
  normal glide so that it activates on a square press". KH1's glide starts on Square held while
  falling (KH2's air, or KH1's jump), once a hold; the jump press in KH1's air is KH2's (Aerial
  Dodge). In KH1's glide, Square held is fed to KH1's control step as its jump held.
- "unable to input a kh1 dodge roll": s240 read KH1's ability bits at `0x142F5F610`; they are at
  `0x142D5F610` (Steam `0x142D5EC10`, gen2's override fixed).
- "unable to input a kh2 glide when glide is set to KH2": KH1's glide style zeroed KH2's glide
  level in Sora's abilities, which are only set again when KH1's menu changes; kept aside now and
  given back.
- The user: "an option to switch between kh1/kh2 battle quotes". `styles::Kind::BattleQuotes`
  (ini `battle_quotes`, default from `kh2_voice`, so KH1's), every form; KH2's motion voices all
  load and play by it (`kh2sound::kh2_voice`). Config rows go 8 to a page now: Battle Quotes is
  on a third page (the help line reads it as the second page's row in its place).

### 2026-10-19: Stage 243 (KH1's own jump out of its air attack's landing)
- The user (s241): after a KH1 air attack, a high jump in its landing still launched him, and with
  the stick tilted KH2's run took the jump; not after KH1 magic. The log: KH1's state 0 (can't
  act), then state 1 "can act" on the floor: KH1 jumped him itself (its air attack's landing reads
  the jump press outside its control step, where it is kept from KH1), KH1's high jump; KH2 then
  took him rising 25 a frame (`air_free`, its rise carried), or on the stick at state 1 into its
  run with no jump. Now KH1's take-off out of one of its commands (state 1 on the floor, not the
  Jump style's) hands him to KH2 at once with KH2's jump (`kh1_took_off`), KH1's air fields reset.

### 2026-10-20: Stage 244 (Config menu crash)
- The user's log (s242): a crash entering the Config menu, executing 0x40 from KH1's window list
  step (Steam `exe+2E3E2B`, a window's function pointer). s242's 17th row: KH1's list takes 16
  at most, a 17th writes past the window's row arrays. Back to 16 rows (KH1's 8, our 8 places);
  our page scrolls when it has more (`SCROLL`): Down on its last place / Up on its first moves
  our rows instead of the cursor (KH1's list masks: up `0x10000010`, down `0x40000040`, set at
  `0x1402ED1E0`). Battle Quotes is the 9th, scrolled in below Dodge Roll.

### 2026-10-20: Stage 245 (the user's s244 list: KH1 roll, glide, jump, Drive in KH1 combos)
- KH1 roll (helper, `roll-decouple`): "soras model decouples". KH1's placement (model matrix
  `+0x260`) lags his position (`+0x10`) over its roll and some frames after (its root
  compensation, `0x14029CF10`, read, not confirmed); our draws sat up to ~300 behind him, then
  snapped. Over KH1's roll and 0.6 s after it the drawn placement is his position
  (`pose::on_actor`, logged), and no stale KH2 pose is drawn over the roll's start.
- KH1 jump / glide (`stylejump`) hand-offs strip KH1's jump and Square (they weren't: KH1 had him in
  states 1 / 2 / 21 without its can-act, so `strip` was off): no KH1 glide on the jump button in
  KH1's jump ("still able to do a normal kh1 glide by pressing the jump button"); in KH1's glide
  Square held is its jump held, so the glide holds ("go into the normal kh1 glide for a second
  then get forced out": KH1 ended it after 8 frames, seeing jump up).
- Drive in KH1's own combo: "unable to select drive (it is blacked out)". The cut window
  (`KH1_WINDOW`) is open all through a KH1 combo for a Drive (KH2 takes him to change); its Attack
  stays KH1's (`combat::run_entry`).

### 2026-10-20: Stage 246 (KH2 Thunder's sounds; KH2 battle quotes on every move)
- "KH2 thunder doesnt have any sound FX": the PC log, "KH2 sound 126: scd: no stream for sound
  205". se000's stream table holds 215 streams (`0x34`); the reader took `0x32` (152), so
  Thunder's (126 / 127 / 128 / 148 on entries 205..208) had none. `kh2_formats::scd` reads the
  larger. (Seen meanwhile: the "entries" table read with `0x30`'s 314 runs into the stream table
  past 152, so Reflect's 138 / 139 / 176 (entries 180..184) play streams 28..32 through it, not
  streams 180..184; left as is, the user has heard them since s178.)
- "when switching to KH2 battle quotes, there are no KH2 battle quotes": only the `kh2_sounds`
  list's moves (ability moves by default) had voices loaded and played. Every move's voices load,
  and play on any move with Battle Quotes at KH2; KH1's swing voice stands down for every move
  then.

### 2026-10-20: Stage 247 (KH1's glide bit read as Square)
- The user (s245 log: Jump KH2, Glide KH1, Dodge Roll KH1): jumps cut short of their full
  height ("multiple inputs are being sent ... unable to get the max height"), straight into KH1's
  glide, "still able to input normal glide on the jump button". `takeover::buttons` reads KH1's
  action bit `0x100000` of its held state as Square, and KH1 sets it too with its jump held in
  the air (its glide): KH2 saw Square mid-rise (the rise cut from 14 to 2 a frame at ~126) and
  the Glide style's start (Square held, falling) fired at the top. With the jump held, only the
  pad's own Square counts now (logged when the bit comes with the jump).

### 2026-10-20: Stage 248 (KH1's jump: its height back, no glide on the held jump)
- The user (Jump KH1): "as soon as i jump i go into a glide. I also dont as high of a jump as i
  should be getting because it cuts off the input to the jump button". s245 kept KH1's jump bits
  from it all through its jump, so its jump read as released at once (a short hop). Now while
  KH1's jump rises (its state 1, his height going up) its jump held is KH1's (held only, not the
  presses); from the top on it isn't, so holding it doesn't glide (the glide, Glide at KH1, is on
  Square). The glide right after the jump in s246 was also s247's Square bit.

### 2026-10-20: Stage 249 (no input buffers or queues)
- The user: "comment out any code that is related to queueing/buffering moves? its currently
  breaking a lot of situations". Commented out / switched off, a press counting only on its own
  frame:
  - s241's jump latch (`GROUND_JUMP`: a press in KH1's move kept for its end) and its forced jump
    on KH2's first frame with input (`PENDING_JUMP`); KH2's held state at the hand-off
    (`JUMP_HELD_OVER`) and KH1's own take-off turned into KH2's (`kh1_took_off`) stay.
  - s234's Attack carried through frozen frames (`CARRY_MAX`): dropped at once.
  - `combat::PENDING_COMMAND` waits: a command KH2's move can't take now is refused (the
    menu's "can't"), not kept for its next window; one chosen in KH1's jump / glide / roll is
    taken only if KH1 lets him act on that frame; a spell during KH1's cast only while it could
    cut it now (`takeover::SPELL_CUT_OPEN`), else refused / dropped; what's stored is dropped
    after 0.2 s.
  - KH2 spells' and Reflect's `WAITING` (up to 5 s for the move's next window): dropped at once.

### 2026-10-20: Stage 250 (the dream weapons' stretch, again)
- A tester (the user's relay, Dive to the Heart, screenshots): "tap move, the split second any
  animations begin, he stretches ... his un-stretched shield appears at the end of the stretched
  mesh", not on jumps; a long stretched blade along the floor. s226's rigid hold (`dream_hold`)
  notes KH1's hold from KH1's pose on any frame KH1 posed him; on an animation's first frame some
  weapon bones were still the last drawn ones, far from the hand, and that hold put the weapon
  there. A hold is noted only with every weapon bone within 250 of the grip (logged otherwise).
  The shield's orientation ("isn't perfectly oriented in his hand") is open: no detail given.

### 2026-10-20: Stage 251 (KH1's jump handling off for now)
- The user: comment off, for now, (1) the Jump style's KH1 jump start (`style_air_start`'s jump
  branch: KH1's ground jump handler, KH2's speed into its air speed, its take-off height), (2)
  the jump button kept from KH1 in its jump / held while rising (s245's strip for
  `stylejump::JUMP`, s248's `kh1_rise`; KH1's glide keeps its strip and Square-as-jump-held),
  (3) s247's filter on KH1's action bit (it reads as Square again, as before). The Jump row at
  KH1 does nothing meanwhile (KH2's jump).

### 2026-10-20: Stage 252 (Gummi Routes skip, our own; Misc Animations)
- The user: "can you put an option in the config to skip the gummi routes?", then "no dont use
  their script, recreate it": KH-1FM-AP-LUA's Instant Gummi (s114-s138) not restored. A helper
  (`gummi-own`, `gummi.rs`, `docs/kh1-gummi.md`) read the executable: the world map's task
  (`0x140219490`) starts a route through `0x14020EF10 (kind, stage)`; a cleared flight
  (`0x14023DA40`) lands through KH1's landing list `0x140213B20 (world)`, `0x140266920 (4)`,
  `0x14020ED90 (landing)`. With `gummi = skip` the route start is hooked to run those landing
  calls for the chosen world (`0x140507C94`) at once; story-started flights (`0x14020EEB0`) are
  left. Config row "Gummi Routes" (Vanilla | Skip), our last. Steam: data overrides
  `0x507C94`→`0x506F94`, `0x50B1B4`→`0x50A4D4`.
- The user: "a setting for misc animations to be KH1 or KH2? Like item throwing animation"
  (helper, `misc-anims`): `styles::Kind::MiscAnims` (ini `misc_anims`, default kh1), its row
  before Battle Quotes. At KH2, KH1's item use is drawn with KH2's motion 9 (P_EX100 A110, picked
  by its shape: the hand overhead at 20-30, a voice at 0; unconfirmed it is KH2's item use);
  KH1 still runs the item. Not mapped: summons, examining, picking up.

### 2026-10-20: Stage 253 (why KH1 keeps him: logged)
- The user: "it reverted me to KH1 Animations right after i defeated the card tower" (Wonderland,
  the PC's s249 log). After the boss's scene the log says nothing of who had Sora: "KH1 has
  Sora (why)" stopped at 400 lines (shared with the status lines) long before. A new reason is
  now always logged (at most 4 a second), and while KH1 keeps him outside its own moves for over
  5 s a status line every 10 s: the reason, KH1's state / slot / flags, the event and script
  values, whether his body was seen, takeover on.

### 2026-10-20: Stage 254 (Misc Animations: standing, walking, running too)
- The user: "misc animations: KH1 — sora doesn't switch to kh1s misc animations when kh1 is
  selected" (only the item use was mapped, none in the log); asked, "idle walk/run plus items".
  At KH1, while KH2 stands / walks / runs him on the ground and KH1's slot is one of its own
  0..7 (KH1's control step picks its idle / walk / run from the stick under KH2 all along), KH1's
  own animation is drawn (`takeover::MISC_KH1`, `pose::wanted`, blended in from our last pose),
  KH1's step sounds play and KH2's footsteps don't; KH2 still moves him. The default is now KH2
  (KH2's walk and run as before; KH2's item motion with it).

### 2026-10-20: Stage 255 (Jump at KH1: KH1's jump animations)
- The user: "turning jump to kh1 doesn't change soras jump animation back to kh1" (s251 took
  KH1's own jump out); asked: KH1 animations only, KH2's physics. With Jump at KH1 (base Sora),
  KH2's Jump / Air / Land are drawn with KH1's own motions, evaluated from KH1 Sora's motion
  table (`xa_ex_0010.mset`, `kh1_data::mset`) on his skeleton (`pose::kh1_air_pose`): 12 the
  take-off then 13 the rise, 10 the fall (looping), 11 the landing (held at its end); timed from
  when KH2's state got there (`takeover::KH1_AIR_SLOT`). KH2's Keyblade turn and wrist aren't
  applied over them (`NATIVE_KH1`). Slots from the play logs (s16x), not checked frame by frame.

### 2026-10-20: Stage 256 (KH1's own jump back on)
- The user: "can you actually go with the number 2 option" (asked at s255: turn KH1's jump back
  on). s251 undone: the Jump style's KH1 jump start (KH1's ground jump handler, KH2's speed and
  the take-off height), its jump held KH1's while rising (s248) and kept from it from the top,
  KH1's jump / Square kept from it in its jump (s245), and s247's filter on KH1's action bit.
  s255's KH1 animations over KH2's jump stay for KH2's own falls with Jump at KH1 (after a KH2
  move in the air).

### 2026-10-20: Stage 257 (KH2's player model)
- The user: "an option in config to switch between sora's player model And his KH2 Player
  Model"; asked: KH2's model always (KH1's animation carried onto it), the forms' own KH2 models,
  KH2's Keyblades. Helper (`kh2-model`, `docs/kh1-kh2-model.md`): `kh2_formats::mdlx` (meshes,
  skeleton, textures replayed through the PS2 GS memory layout), `kh2_anim::skin`,
  `kh1_data::inverse` (KH1's pose onto KH2's skeleton), `win/model.rs` (loads P_EX100 and the
  forms' models and the matching KH2 Keyblades from the user's KH2 install, hides KH1's Sora by
  masking his body meshes in the model queue, draws the KH2 model in the effect renderer before
  the effects, with depth and alpha cut-out). Config row "Player Model" (KH1 | KH2), ini
  `player_model`, default kh1. No lighting yet; KH1's Keyblade while KH1 animates him; nothing
  seen in game. The model-queue hook is always installed now (`valor_left_blade` only gates the
  second Keyblade).

### 2026-10-20: Stage 258 (Valor pick's Strength, again)
- The user: "the strength cap is at 70 again" (asked: the Valor Keyblade pick). KH1's preview
  puts a Keyblade in Sora's weapon byte a hover behind, and at times still his own; s223 took
  the byte for the hovered one and added nothing when it was his own, so the stats shown were
  his with his own Keyblade alone. Now: KH1's figure minus the byte's Keyblade, plus his own and
  the one under the cursor (`equipmenu::valor_preview` returns all three, the hovered row from
  the list).

### 2026-10-20: Stage 259 (Player Model off for now)
- The user: the KH2 player model "comment it out for now" (s257's bugs being fixed meanwhile).
  Its Config row is commented out (`config::ROW_KINDS` maps rows to styles explicitly now) and
  `model::on` is false whatever the ini says; the code stays.

### 2026-10-20: Stage 260 (KH2 player model fixes, still off)
- The user's s257 list ("doesnt show up in pause menu ... flickers pretty consistently ...
  doesnt have any VFX for his combos ... KH1 sora animations is super contorted"), helper
  (`kh2-model-2`, `docs/kh1-kh2-model.md`): the pause menu's own Sora copy drawn as KH2's model
  (its KH1 pose carried over, KH1's copy masked, KH1's menu camera); the flicker (the log:
  hidden 18000 times, drawn 9126): the draw now prepared when KH1's Sora is held back and drawn
  on every world pass that hid him, never hidden without it, locks no longer skipped, KH1's
  interpolated-camera frame replay (`0x140102740`, hooked, counted) shows KH1's model while it
  runs, a 20 s counter line; the combo effects: KH1's Keyblade stays queued (shrunk into the
  grip for its draw) so its trail and effects carry on; the contortion: `kh1_data::inverse`
  rebuilt on the forward retarget's body parts (limb directions and knee / elbow roll from KH1's
  joint positions, torso turns blended, KH2's own rest for hair / face / hood / cloth). The
  option stays commented out (s259) until the user turns it back on.

### 2026-10-20: Stage 261 (testing branch: shared states)
- Testing branch only. The user: "lets rewrite how kh2 injects it animations into the game ...
  The goal is to cleanly separate animation handling from the unique logic and physics each
  version of Sora needs". Also "comment the jump codes I just asked you to restore": s251's
  comment-off re-applied (KH1's jump start under KH2, the jump-button strip, s247's filter).
- Every move style is KH1's by default (`styles`, `settings`; an ini that already has a key
  keeps it).
- `shared.rs` (`docs/shared-states.md`, ini `shared = on`): for base Sora, KH1's own state
  machine owns him. Idle, walk, run, item use, jump, high jump, fall and landing run in KH1's
  states:
  - at KH1, unmodded;
  - at KH2, KH1's slots are drawn with KH2's motions (`shared::kh2_look`, `pose::follow`) and
    moved by KH2's numbers (`shared::physics`).

  The physics wraps KH1's state table (`0x142DB4240`): the ground step's displacement is
  scaled to KH2's walk or run speed. KH1's takeoff and its state 2 step follow KH2's jump or
  High Jump arc, gravity, release hop, fall cap and air steering, with KH1's own arc fields
  kept in step.
- KH2's controller only borrows him for KH2's moves (`takeover::shared_frame`):
  - Aerial Dodge, as a double jump with Jump at KH1 too;
  - KH2's glide;
  - Square at Dodge Roll KH2;
  - Attack at a combo style KH2;
  - KH2 spells, Reflect, Drives.

  It gives him back to KH1's ground or fall state when the move is over
  (`shared::give_back`). A ground jump out of a KH2 move is KH1's own.
- Removed: KH1's idle / walk / run and jump drawn back over KH2's (`MISC_KH1`,
  `KH1_AIR_SLOT`, `pose::kh1_air_pose`).
- RE (helper, static): KH1's jump is a 4-frame crouch in state 1 (`0x1402ADC10`), then the arc
  in state 2's step `0x1402AD510` (the same sin-sin rise as KH2's, then `g/2 t²`, capped at 20);
  heights from btltbl (190 / 290); walk 2.0 / run 8.0 in the stats' `+4` / `+8` (stick past
  0.8); landing by collision (`+0x2D0` bit 0).
- Not yet: Base / Aerial Combo, Magic and Dodge Roll at KH2 are KH2 borrows, not KH1's states
  with KH2's motions. Nothing tested in game.

### 2026-10-20: Stage 284 (testing branch: shared combo and glide, KH2 abilities on KH1's moves, split ability ids, 30 fps, KH2's camera in KH1's)
- The user's testing list (s261, 30 fps Steam log): shared animations for base attack 2, the
  finishers and glide; KH2's dodge roll unusable; KH1's roll into Quick Run; KH2's combo,
  Square and general modifiers with the combo styles at KH1; KH2 in slow motion at 30 fps; KH1's
  camera calls taking the camera; split KH1's and KH2's ability ids; docs.
- **Shared ground combo and glide** (`shared.rs`, `pose.rs`): KH1's combo rows and glide state at
  either style, drawn with KH2's swings (98 -> 151, 99 -> 152, 101 -> 154) and glides (113..116)
  at KH2, with KH2's travel and glide numbers. KH2's dodge roll: s261's log had Dodge Roll, Guard
  and Quick Run unequipped (nothing for KH2's Square); Square now borrows only with one of them.
  KH1's roll held into KH2's Quick Run (Quick Run equipped).
- **KH2's abilities on KH1's moves** (`combomods.rs`): KH2 borrows him where it would use an
  ability move (openers, steps, finishers, Square moves, Negative Combo / Finishing Plus,
  Aerial Recovery, Retaliating Slash), KH1's own rows elsewhere.
- **Ability ids split** (`growth.rs`, `menu.rs`, `docs/abilities.md`): KH1's 1..65 vanilla, KH2's
  moves on 78..87 (66..77 as before); an owner table; old saves moved once (mark at save
  `+0x48A`).
- **30 fps** (`takeover::steps`): KH2's side runs whole 60 Hz steps a game frame.
- **Camera** (`camera.rs`, `docs/kh1-kh2-camera.md`): KH2's camera as KH1's own modes; KH1's ledge
  / climb cameras, its lock-on blends and its "held still" scene test no longer take it;
  `camera_calls = kh2` (ini).
