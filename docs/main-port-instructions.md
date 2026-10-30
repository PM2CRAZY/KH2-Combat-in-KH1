# Porting the main branch into testing (before the merge)

Instructions for any agent (or person) moving what the main branch has onto the `testing` branch. Written
by the owner's request, 2026-10-29. Docs only: this file does not bump the version.

**What this is.** The plan is for `testing` to replace `main` later. Before that can happen, every feature
a main-branch player has must exist on `testing`, built the testing way (native, 1:1, KH1 stays whole), and
a main player's ini and save must carry over. **This is not the merge.** Nobody merges, opens a PR against,
or pushes to `main` until the owner says so in plain words. All work in this file lands on `testing` only.

**Read first, in this order:** `docs/design-philosophy-instructions.md`, `docs/engineering-rules.md`,
`docs/project-instructions.md`, `docs/tester-requests.md`, the top ~15 stages of `AGENTS.md`, then this
file. If this file and those differ on anything except section 1 below, those files win.

---

## 1. The one exception: reading main

`docs/project-instructions.md` says not to pull code or data from main. For this port only, the owner
allows **reading** main, as a specification, never as a source:

- **Read main to learn:** what a feature does in game, its ini keys and defaults, its Config rows, the
  owner's decisions about it (quoted as "the user: ..." in main's `AGENTS.md` stages and module headers),
  and its research notes in main's `docs/`.
- **Never copy from main:** no code, no structure, no addresses, no tables. Rebuild each feature on
  `testing` from the KH1 Steam decomp, the KH2 Steam decomp, the OG Rust rewrite and the owner's
  decisions. Main's code ran KH2's controller beside KH1 ("running along"); a straight copy brings that
  back.
- **Main's addresses are Epic.** Main writes every game address as an Epic RVA (`Rva(0x...)`) and maps
  it to Steam through its `steam_map.in` (main `docs/steam-epic-addresses.md`), even the ones it found on
  Steam. So a `0x14...` in main's code or docs is the Epic address. Use main's `steam_map.in` only as a
  hint for where to look on Steam; then find the Steam function in the Steam decomp, match its logic,
  read its first bytes from the Steam exe, and hook only on an exact match.
- **Main's "Checked" tags don't carry over.** A finding main marked Checked (exe) was checked on Epic and
  under KH2's controller. On `testing` it is an **Assumption** until you check it again on Steam.

**Which main, and keeping the versions aligned.** The `KH2-Combat-in-KH1-main` folder inside the
TESTBRANCH repo is the owner's local copy of main (not tracked by git). This file was written against
**main s339** and **testing t923** (2026-10-29). Main keeps moving, so:

- **Before every session of port work,** read `VERSION` in both `crates/kh1-hook/src/lib.rs` files (main's
  copy and `testing`'s) and the top stage of both `AGENTS.md` files. Also check `origin/main`: if it is
  newer than the local copy, tell the owner (he refreshes the copy) and read `origin/main` read-only
  (`git show origin/main:<path>`, never a checkout over `testing`).
- **If main is newer than the version in `docs/main-port-inventory.md`,** read every main stage after it
  before porting anything: each new stage is either a new inventory row, a change to an existing row
  (re-open its research round if it changes the spec), or a bug fix only for main's deprecated path
  (mark it in the row). Then set the inventory's "aligned with" line to the new main version.
- **Testing's version** is the base for every change (never roll back). Note it beside the main version
  in the inventory's "aligned with" line, e.g. `aligned with main s339 / testing t923`.
- A main stage can change a decision the port already built (s331 changed Limit Form's cost after the
  gauge was designed). If a ported row's spec changed on main, say so and ask before changing the
  ported version.

## 2. Rules that still apply to every ported feature

Short reminders. The full text is in the docs named above.

- **Native, never deprecated (section 2a).** KH1's engine runs it: new states, table rows, requests,
  motion slots, effect programs, sound banks, HUD pieces, prize types, item records, functions patched
  into KH1's own. Not our own object or renderer beside KH1's copying results into KH1's fields, and
  never one of the deprecated systems in section 2a.
- **KH1 stays whole.** With every row on KH1, the game plays exactly like vanilla KH1. A ported feature
  that changes KH1's feel gets a Config row whose KH1 value is vanilla.
- **1:1.** Any KH2 behavior uses KH2's numbers (KH2 decomp or the owner's trace). Features that were the
  owner's own design on main (not in KH2: Drive Recovery sold in Traverse Town, KH1 limits in Limit Form,
  the gauge maximum table) keep the owner's rules exactly as main's stages quote them, unless a later
  `testing` decision changed them (section 4).
- **Little of ours per frame; assets built once.** Prefer data KH1 reads over per-frame hooks. KH2 art
  (the Drive gauge HUD, the Drive orb sprite) is built once from the player's own extracted files into the
  mod folder, not decoded in memory at every start as main did.
- **Steam only, no decompiled code or game files in the repo, don't break KH1's events and scripts.**
- **Versions.** One system per version, `t` + next number after the current top stage, `VERSION` in
  `crates/kh1-hook/src/lib.rs` plus a matching `AGENTS.md` stage. Run `cargo test` before pushing.
- **Never roll back.** Read `VERSION` and the top `AGENTS.md` stage before every change and build on them.
- **Mark assumptions.** Every claim about KH1 or KH2 gets a tag: Checked (exe / decomp / files / trace)
  or Assumption.

## 2a. Never port a deprecated system; everything native and 1:1

These three rules have no exceptions in this port.

**1. Nothing deprecated comes over.** Main is built on systems `testing` has deprecated. None of them
is ported, called, re-enabled or rebuilt under a new name, even for one feature, even "for now", and
even if it would be faster:

- KH2's controller taking Sora (`character-core`'s controller, `takeover`'s physics step replacing
  KH1's, the Driver, `form::character` / `form::battle` swapping in KH2's moveset);
- our own hit sweeps or strike checks instead of KH1's attack objects and hit pass;
- our own spell or form script runtime (`kh2-magic`, `kh2-bdx` running KH2's scripts) instead of KH1's
  own commands, states and attack objects;
- our own renderer, player or effect system beside KH1's (KH1's renderer is extended instead, as the
  effects work did);
- per-frame hooks that steer KH1 into a result, where KH1 could read the result as data (rows,
  requests, tables, motion slots, prize types, item records);
- KH2 assets decoded in memory at every start (they are built once into the mod folder);
- anything else the inventory marks as "running along".

A main feature that only works on one of these is ported by rebuilding what it **does** on KH1's
engine, not by bringing the deprecated part along. A main fix that only existed because of one of
these (the submenu-jump fix, the water and pit fixes, the no-damage time while KH2 had Sora) is
**Obsolete**: confirm in game that KH1's own systems already behave right, and report it.
If a `testing` system the port would build on is itself still on a deprecated path (Final, Master,
Limit and Anti on KH2's controller, for example), don't build on it: say so and ask.

**2. Everything is native and 1:1.** Native: KH1's engine steps, draws, plays, collides and ends it.
1:1: every KH2 value matches the Steam KH2 decomp (section 2b) or the owner's trace. A port that is
native but not 1:1, or 1:1 but not native, isn't done.

**3. If KH1's engine can't do it, give KH1 the ability.** When a feature needs something KH1 has no
system for (a gauge, a new prize type, a new item kind, a menu page, a HUD piece, a state, a request,
a camera mode, a sound bank, a save field), build that capability into KH1's engine: new rows, states,
table entries, records, functions patched into KH1's own, so KH1 itself runs it. Don't work around the
gap, don't fake it with an overlay or a parallel system, and don't ask permission to build the
capability (that is part of the task). Ask only when (a) after a real attempt the native result still
can't match KH2 1:1 (say what differs, why, and the native options), or (b) there are two native
designs that play differently. Write the new capability up in its system's doc so later ports reuse
it instead of building a second one.

## 2b. The Steam decomp check (required for every ported system)

Main's version of a feature is not proof that it was 1:1 or complete. Main was built on the Epic exe,
under KH2's controller, and often in steps the owner asked for one at a time. So every system ported from
main is checked against the **Steam C decomps** (`D:\claude\ghidra_12.1.4_PUBLIC_20260921\DECOMPS`: the
Steam KH1 and Steam KH2 exports), on two counts:

1. **Is it 1:1?** Every number, timing, rule and condition the port uses (costs, rates, lengths,
   windows, caps, drop rates, orb values, menu behavior, sounds played, when it shows or hides) is
   found in the KH2 Steam decomp (or the OG Rust rewrite, then confirmed in the Steam decomp). Each
   value in the research round and the `AGENTS.md` stage says where it came from: the function's Steam
   address and a Checked (decomp) tag. A value only main had is an **Assumption** until found.
2. **Is anything missing?** Read KH2's whole system in the decomp, not just the parts main used: every
   caller and callee of its main functions, every field of its records, every branch and special case
   (a form, a summon, Anti, an event, a cutscene, a game over, a room change, a save, a load, the
   menus, the HUD in each state). List everything KH2 does that main didn't, under "Missing from main"
   in the system's row. Do the same in the KH1 Steam decomp for what the native version touches: every
   KH1 reader of the fields, tables, save bytes and HUD tasks it uses, so nothing KH1 does breaks.

Report both in the research round: "1:1 checked" (value, KH2 address, KH1 address) and "missing from
main" (what it is, where it is, whether it matters for the feel). **Ask the owner before adding a
missing feature** (the scope rule), but always report it. If something can't be found in the decomp,
say so and say where you looked; don't fill the gap with main's value as if it were checked.

Where the system was the owner's own design on main and isn't in KH2 (Drive Recovery sold in Traverse
Town, KH1's limits in Limit Form), there is nothing in KH2 to be 1:1 with: check the KH2 parts it
builds on (KH2's item rules, KH2's Limit menu) and the KH1 side, and say which parts are the owner's
rules.

## 3. Phase 1: the inventory (docs only, no code)

Make `docs/main-port-inventory.md`, one row per main system. Its first line is the "aligned with" line
(`aligned with main sNNN / testing tNNN`, section 1). Nothing is built until the owner has read it.

**How to find every system (don't stop at the seed list in section 6):**

1. Every module in main's `crates/kh1-hook/src/lib.rs` and `src/win/mod.rs`.
2. Every key in main's default ini (`settings.rs`, `DEFAULT_INI`) and every Config menu row main adds
   (`config.rs`, `menu.rs`).
3. Every file in main's `docs/`, and the files that exist on main but not on `testing` in the other
   crates (`kh2-formats`, `kh2-data`, `kh1-data`, `kh2-magic`, `kh2-rig`, `kh2-anim`, examples and tests
   included).
4. Main's `AGENTS.md` stages: every "the user: ..." quote is a decision. Note the stage number (s123).
5. Main's `README.md` and `crates/kh1-hook/openkh/README.txt`: what players were told the mod does.

**For each system, fill in:**

| Column | What goes in it |
|---|---|
| System | Plain name (e.g. "Drive gauge") |
| On main | Files, docs, ini keys, Config rows, the stages that built it |
| What it does | Plain words, what the player sees |
| Owner's decisions | Quotes with their main stage number |
| On testing now | One of: **Ported (native)**, **Ported (running along)**, **Partly**, **Missing**, **Superseded** (a later owner decision replaced it: quote it), **Obsolete** (only existed because KH2's controller had Sora; say why it no longer applies) |
| Evidence | The `testing` file, version or doc that shows the status |
| Conflicts | Where main's design and a `testing` decision disagree |
| Decomp check | Not started / done: the 1:1 values checked in the Steam decomps, and what main was missing (section 2b) |
| Plan | One line per version it would take |

Status rules: "Ported" needs evidence in `testing`'s code, not a similar name. "Obsolete" and
"Superseded" need a reason the owner can check; when unsure, mark it **Missing** and ask. Anything main
does that is related but not a module of its own (a fix inside `takeover.rs`, a special case in
`combat.rs`) is a row too, if a player would notice it gone.

Then send the owner a short summary in chat: counts per status, the conflicts, and the questions.

## 4. Phase 2: one research round per system

For each **Missing**, **Partly** or **Ported (running along)** row, before any code, write a research
round in `docs/main-port-inventory.md` under that system (and in the system's own doc in `docs/` for the
findings), in the same shape the testers get:

- what `testing` has now; what main did and what the owner decided there;
- what KH2 does (if it is a KH2 feature), tagged, read in the KH2 Steam decomp (section 2b);
- how KH1 does it on Steam, tagged (the functions, tables, save bytes it would use), read in the KH1
  Steam decomp;
- "1:1 checked": each value the port will use, with its Steam address; anything only main had, marked
  Assumption;
- "Missing from main": what KH2 does in this system that main never had, and whether it matters;
- what main's version ran on that is deprecated (section 2a), and what replaces it on KH1's engine;
- what KH1's engine lacks for this system, and the capability you'll add to it;
- the native plan, one version per part;
- what you couldn't find yet;
- numbered questions with options (a), (b), (c), each saying what changes, with your recommendation.

**Conflicts you must raise, not settle:**

- A main decision against a later `testing` decision (example: main's forms cost Drive bars; `testing`'s
  forms cost 3 MP and drain MP, and the MP system is still undecided).
- A main behavior that changes KH1's feel with no KH1 setting (it needs a Config row: ask which list
  it goes in and what it's called).
- A main ini key whose name `testing` reuses with another meaning (see section 5, `physics`).
- Save bytes main used that `testing` also uses (see section 5).

**Go-ahead.** These aren't tester threads: the owner is the requester and this file is the request. No
code for a system until the owner writes "go ahead" in chat (or in a thread, if he opens one) after your
research round for it. If a tester thread already covers the same system, follow the thread and tell the
owner.

## 5. Making a main player's setup carry over

### 5.1 The ini

Main's `kh1-kh2.ini` must load on `testing` without the player losing their choices. First-pass mapping
(**every row is an Assumption until checked against both `settings.rs` files**):

| Main key | On testing | Conversion |
|---|---|---|
| abilities, hurt, carry, ceilings, grabs, commands, swing_fx, kh2_sounds, kh2_volume, damage_scale, glide, air_cast_fall, kh2_voice, auto_aim, camera, camera_scale, camera_lower, config_row, shortcuts, kh2_effects, flip_weapons, reflect, keyblade_turn, drive, valor_left_blade, ground_combo, air_combo, jump_style, glide_style, roll_style, misc_anims, player_model, battle_quotes, gummi | same key | Keep. Check each value still means the same thing. |
| `physics` (on/off: hair, hood and chain sway) | `physics` exists with **another meaning** (kh1/kh2: whose movement physics) | **Drop main's line.** Must not be read as the new key. |
| `kh2_hair` | none | Drop: hair, hood and chain are baked into the animations now (owner). |
| `walls` (kh1 / clink / pass) | `wall_clink` (on/off) | clink → on; kh1 and pass → check what `wall_clink = off` does against main's kh1 and pass before mapping. |
| `music_mute` (lowest / on / off) | `volume_mute` (on/off, every slider, t915) | lowest → on, off → off; on (music always silent) has no equivalent: ask. |
| `magic`, `air_magic` | per-spell `magic_*`, `magic_air_physics` | `testing` already reads the old two (m19); confirm. |
| `magic_scale` | none | Superseded: KH2 damage runs through KH1's magic stat with no KH2 bonus on top (owner). Drop. |
| `bonus_ap` | none | Ask: port or drop. |
| `hud_toggle_key` | none | Comes with the HUD hide port. |
| `drive_gauge_art`, `drive_gauge`, `drive_gauge_scale`, `hide_hp_label`, `drive_gauge_debug` | none | Come with the Drive gauge HUD port. |
| `found` (written by the mod) | `found` | Check it's the same record. |

When the inventory is done, the converter is its own version near the end: on load, a key only main
writes (or a main value) is converted to `testing`'s key, the player's choice kept, dead keys dropped,
a one-time backup of the old ini written beside it, and one log line per converted key. **Ask the owner
before building it** (convert in place, or start a fresh ini and keep the old one as a backup).

### 5.2 The save

Main wrote mod data into the player's save. A main save must load on `testing` with nothing lost and
nothing misread:

- **Drive gauge:** bars at save block `+0x37` and points at `+0x74 + 0x37` (the padding byte after the
  weapon in Sora's and Donald's party records; main `drivegauge.rs`). Check these bytes are free on
  `testing` (form levels and form EXP may already use party-record padding) before choosing where the
  native gauge keeps its data. If they clash, ask.
- **KH2 abilities:** main's `abilities` modes (including `remove`) write KH2 ability ids into the save.
  `testing` swaps the ability list by setting and writes that to the save. Check a save made on main
  with KH2 abilities learned loads correctly on `testing` in both settings (Glide style KH1 and KH2).
- **Items:** main's Drive Recovery items are item ids `0x91` / `0x92` with counts in KH1's inventory.
  The port must keep those ids, or convert the counts.
- Anything else the inventory finds main writing to the save goes in the same list.

Write what you find in `docs/main-port-inventory.md`, section "Save". Ask the owner before any code that
converts a save.

## 6. Seed list (a first pass, not checked: verify every line)

Found by comparing main s339's file lists, module headers, ini and `AGENTS.md` stages against `testing`
t923. Use it as a
starting point for section 3, not as the answer.

| Main system | Main files | First-pass status on testing |
|---|---|---|
| Drive gauge (bars, points, max 3 +1 per six fights, costs Valor 3 / Wisdom 3 / Limit 4 / Master 4 / Final 5 / Anti 9; fills on hits, damage taken, orbs; form gauge `(level + 3) × 600`) | `drivegauge.rs`, `win/gauge.rs`, main `docs/drive-model.md` "The Drive gauge in KH1" | **Missing.** `testing` forms cost 3 MP. Already planned as step 1 in `docs/kh1-drive-forms-native.md` §5. Conflict: MP vs gauge. |
| Drive gauge HUD (KH1 pieces, and KH2's own art from `zz0field.2dd`) | `win/gaugehud.rs`, `win/kh2hud.rs`, `kh2-formats` `seqd.rs`, `dds.rs`, `png.rs` | **Missing.** Planned step 2. Art must be built once into the mod folder. |
| Drive orbs as KH1 prize types 7 / 8, and KH2's orb art | `win/prize.rs`, `win/prizeart.rs`, `kh2-formats` `prize.rs`, main `docs/kh1-prizes.md` | **Missing.** Planned step 4. Already the native shape on main (KH1's prize code runs them); re-find on Steam. |
| Drive Recovery / High Drive Recovery items: sold in any item shop with a Potion (Traverse Town, Agrabah), dropped by enemies (`driveitems::TIERS`, Lucky Strike applies) and in 17 chests (s333), once a form is open | `driveitems.rs`, `win/items.rs`, main `docs/kh1-items.md` | **Missing.** The drop rates and chest list are the owner's design (s333), not KH2's. |
| KH2 support abilities: Damage Drive, Form Boost, Damage Control, Drive Converter, learned at Sora's level (14 / 24 / 38 / 35), KH2's AP and rules; Berserk Charge and Drive Boost left out (no MP Charge in KH1) | `support.rs`, `growth.rs` `SUPPORT_LEVELS`, `win/hurt.rs`, `win/gauge.rs`, `win/prize.rs` (s335, s337) | **Missing.** Needs the Drive gauge first. Drive Boost and Berserk Charge: ask whether KH1 gets MP Charge (the MP question). |
| Forced reverts (KO, new world, scenes, battle end, Coliseum arena) refill every Drive bar and bring partners fully back; Revert command and gauge-out don't | `win/form.rs` `revert(forced)`, `gauge::refill_bars`, `vanish::force_back` (s338) | **Missing** (comes with the gauge and partners vanish). |
| Drive Forms across Olympus Coliseum rounds: reverted on arena entry, kept through a won round, no drain in the victory scene | `win/cup.rs`, main `docs/drive-model.md` "Drive Forms in the Coliseum" (s332, s334) | **Missing.** Check against KH1's own cup scripts on Steam. |
| KH2-style Cure heals by KH2's rule (`ceil(power × Magic / 100)`, partners half), KH1's max MP × 3 × `magic_scale` standing in for Magic | `cure.rs`, `win/formmagic.rs` (s339) | **Check** against `testing`'s `magic_cure` row. Conflict: `testing` has no `magic_scale` and runs KH2 damage through KH1's own magic stat with no bonus (owner): ask how Cure's Magic stand-in fits that. |
| Story flags: when each form opens, Reflect's levels, the gauge maximum | `story.rs`, main `docs/drive-model.md` "When the forms open" | Check: how `testing` opens forms and Reflect levels now. |
| Forms need their partners (Valor: Goofy, Wisdom: Donald, Master / Final: both) | `partners.rs` | **Missing** (assumption). |
| Partners vanish (and their HUD entry) during the form | `win/vanish.rs`, main `docs/drive-model.md` "Partners vanish in the form" | **Missing** (assumption). |
| Limit Form with KH1's five limits in a Limit menu (KH2's limits kept off); costs 3 bars (s331, KH2's is 4); limits heal per hit with KH2's `hp_drain` values and work out of battle (s336) | `win/limits.rs`, `kh2-magic/tests/limits.rs` | Check: `testing` has no Limit Form as of `kh1-drive-forms-native.md` §1; later stages may differ. |
| Master, Final, Anti Form (and Anti-Sora's look moved onto Sora's HD textures) | `win/form.rs`, `win/tint.rs`, `kh1-data/src/texmap.rs`, `kh2-rig/tests/final_form.rs` | Check. Final stays as it is (owner); the old KH2 controller is deprecated, so these port onto KH1's states. |
| HUD hide on a key (Page Down) | `win/hud.rs`, `keys.rs`, main `docs/kh1-hud-hide.md` | **Missing.** |
| Music fully muted | `win/music.rs`, main `docs/kh1-music-mute.md` | **Superseded** by `volume_mute` (t915), except main's `on` (always silent). |
| Jump press that backs out of a submenu doesn't jump | `menujump.rs`, `win/menujump.rs` | Likely **Obsolete** (KH1 owns Sora's jump on `testing`): confirm in game. |
| No-damage time after a hit while KH2 had Sora | `mercy.rs` | Likely **Obsolete** (KH1's own reactions and flag on `testing`): confirm in `hurt.rs` / `hitreact.rs`. |
| Water, pits and swimming fixes under KH2's control | main `docs/kh1-water-pits.md`, inside `takeover.rs` | Likely **Obsolete**: confirm KH1's own states handle water and pits in every ported move. |
| Bonus AP for KH2 abilities | `bonus_ap` in `growth.rs` | **Missing**: ask. |
| Equipment and equip menu changes (main's `equip.rs`, `equipmenu.rs` are about twice `testing`'s) | `win/equip.rs`, `win/equipmenu.rs`, main `docs/kh1-equipment.md` | Check what main has that `testing` lacks. |
| Reaction commands | main `docs/reaction-commands.md` | Check. |
| Main's other bug fixes in s330–s339 (Neverland flight glide hang, combo rows measuring a target by its body, the grey screen tint during a Drive change, revert limb glows, Counterguard / Flash Step / Retaliating Slash effects, partners left invisible after a cup) | main `AGENTS.md` s330–s339 | Check each: most were fixes to main's KH2-controller path (**Obsolete** if KH1's own states already behave), but the Drive change tint and the effects are KH2 features: check they exist on `testing`. |
| Epic Games Store support | main's `addr.rs`, `steam_map.in`, `docs/steam-epic-addresses.md` | `testing` is Steam-only for new hooks; Epic comes "once the Steam work is done" (`engineering-rules.md`). Merge gate, owner's call. |

## 7. Phase 3: building, in order

After each go-ahead, one system per version. Suggested order (each depends on the ones above it); the
owner may change it:

1. Story flags (forms open, Reflect levels, gauge maximum), if `testing` lacks them.
2. Drive gauge data in KH1's save, KH2's add rules, the hit add at KH1's contact.
3. Forms paid with the gauge (only after the MP question is answered), the form gauge and its drain.
4. The gauge on KH1's HUD with KH1's pieces.
5. KH2's gauge art, built once into the mod folder.
6. Drive orbs as KH1 prize types, then their art.
7. Drive Recovery items: the records and shops, then drops and chests.
8. Partners required, then partners vanish, then forced reverts (bars refilled, partners back).
9. KH2 support abilities (Damage Drive, Form Boost, Damage Control, Drive Converter).
10. Drive Forms across Coliseum rounds.
11. Limit Form (KH1 limits, 3 bars, heal per hit, out of battle), then Master and Anti on KH1's states.
12. KH2 Cure's heal rule (after the Magic stand-in question is answered).
13. HUD hide key.
14. Anything else the inventory turned up.
15. The ini converter, then the save check or converter.

Before calling a system done, compare the built version against the decomp once more: every value it
uses against the Steam KH2 decomp, every KH1 hook point against the Steam KH1 decomp and the exe's
first bytes. If a trace from the owner exists, compare it frame by frame too, and say which of the two
you checked.

After each version, update its row in `docs/main-port-inventory.md` (status, version, what was checked,
what couldn't be) and tell the owner in the usual form: the version names, one line each; "How I
checked it"; "Please test:" as a numbered list; "Still open on my side".

## 8. Before the owner can merge (checklist, nobody acts on it without him)

- Every inventory row is **Ported (native)**, or **Superseded / Obsolete** with the owner's OK. No row
  is "Ported (running along)", and nothing ported calls a deprecated system (section 2a).
- Every ported row's decomp check is done: its values checked in the Steam decomps, and its "missing
  from main" list answered by the owner (added, or left out on purpose).
- A main player's ini and save load on `testing` with their choices and progress kept.
- With every Config row on KH1, the game plays as vanilla KH1.
- `cargo test` passes (apart from the two known kh1-data failures noted in `project-instructions.md`).
- `mod.yml` (title still says "TEST BRANCH"), `README.md` and `crates/kh1-hook/openkh/README.txt`
  describe the merged mod, and say which game builds it supports.
- Open owner decisions answered: Epic support at merge or later; the version letter after the merge
  (keep `t`, a new letter, or back to `s`); whether `main` keeps its history (a merge) or is replaced
  (a `legacy-main` branch kept for the old line); the CI build for `main`.
- The owner uses GitHub Desktop: when the time comes, give him the steps as GitHub Desktop actions.

## 9. Brief for a sub-agent working one system

Copy this, fill the brackets, and give it the files to read:

> You are porting **[system]** from the KH2-Combat-in-KH1 main branch onto the `testing` branch.
> Read `docs/design-philosophy-instructions.md`, `docs/engineering-rules.md`,
> `docs/main-port-instructions.md` (all of it) and the row and research round for **[system]** in
> `docs/main-port-inventory.md`. The owner approved: **[exact go-ahead and answers]**.
> Main (read-only, a spec, never a source): **[main files and docs]**. Main's addresses are Epic: find
> each function in the Steam KH1 decomp (`D:\claude\ghidra_12.1.4_PUBLIC_20260921\DECOMPS`), check its
> first bytes in the Steam exe, refuse to hook on a mismatch. KH2 logic: the OG Rust rewrite, then the
> KH2 Steam decomp. Build it native (KH1's engine runs it), 1:1 where it is KH2's, the owner's rules
> where it is his. Mark every claim Checked (exe / decomp / files / trace) or Assumption.
> Follow section 2a: port nothing deprecated (KH2's controller, our own hit sweeps, script runtime,
> renderer, steering per-frame hooks), and if KH1's engine lacks something the feature needs, build
> that ability into KH1's engine; ask only if native can't reach 1:1 or two native designs play
> differently.
> Do the Steam decomp check in section 2b: confirm every value in the Steam KH2 decomp (main's
> numbers are not proof), read KH2's whole system for anything main never had and list it as
> "missing from main" without building it, and check every KH1 reader of what you touch.
> Work in your own worktree (`git worktree add -b wt-[name] <dir> testing`) and cargo target dir. Don't
> commit to `testing`, push, or touch `VERSION` / `AGENTS.md`. Deliver: one patch per part, the 1:1
> check for each, what you couldn't check, draft `AGENTS.md` stages, the notes you added to `docs/`.
> If a design choice is open, implement what is clear and ask with options (a), (b), (c).

The lead reviews each patch, makes the versions on top of the current `VERSION`, and updates the
inventory.
