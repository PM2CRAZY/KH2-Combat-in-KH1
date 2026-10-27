# Abilities: KH1's ids and KH2's ids (testing branch)

Code: `crates/kh1-hook/src/growth.rs` (the table, learning, migration), `crates/kh1-hook/src/win/menu.rs`
(KH1's ability table, the save, the boxes). Ini: `abilities = kh1`.

## The rule

The user (testing, 2026-10-20): "currently kh2 and kh1 share the same ability ids (one abilitiy
is hurricane blast for kh1 sora while that same ability is explosion for kh2 sora) we need to
split abilities up again into their own separate ability ids, every base ability should be
vanilla kh1 while we add extra ability slots for our kh2 abilities. this is to future proof our
design so that kh2 sora could use exclusive kh1 abilities while kh1 sora can use exclusive kh2
abilities."

- **Every id has one owner and one meaning.** KH1's ids 1 to 65 are KH1's own abilities, with
  KH1's name, help text, AP and effect. Nothing renames them any more.
- **KH2's abilities have their own ids, 66 to 87** (room up to 127), in an extended copy of KH1's
  ability table, with KH2's names, our help text and KH2's AP (from KH2's item table).
- **Each ability says what it does on the other side** (`growth::Side`):
  - for a KH1 ability, while KH2's controller has Sora;
  - for a KH2 ability, while KH1's own states move him (a move style at KH1,
    `docs/shared-states.md`).

  Only what has a clear meaning is mapped today. The rest are `Hook`s, to fill in later.

Before the split (up to testing s261) KH1's ten combo modifiers stood for KH2's nearest one.
Equipping "Explosion" equipped KH1's Hurricane Blast. KH1's own combo (combo style KH1) then
ran Hurricane Blast, and KH2's combo ran Explosion. KH1's High Jump, Glide, Dodge Roll and
Superglide were renamed ("High Jump LV2", "Glide MAX"), and Superglide was turned into Glide.

## The sides (`growth::Side`)

| Side | Meaning |
|---|---|
| `Kh1Own` | KH1's own code, unmodded. On KH2's side, used for KH1 code that isn't about how Sora moves: stats, drops, MP, the HUD, the party, KH1's own commands. KH1 runs these whoever moves him. They have not been checked one by one in the game. |
| `Kh2(effect)` | A value of KH2's controller (`character_core::Abilities`), set while the ability is equipped. |
| `Borrow` | KH1's states own Sora, and KH2's controller borrows him for this move. |
| `Kh1Combo` | KH1's own combo (a combo style at KH1) reads it from `Abilities`. This is `win/combat.rs`'s KH1-style abilities work, done by another helper. Its value is in `Abilities` either way. |
| `Hook` | No meaning yet. |

## The table

`growth::table()` gives one row per id: `id`, `name`, `owner`, `on_kh1` and `on_kh2`. KH1's own
rows are `KH1_ABILITIES`, and ours are `MENU`.

### KH1's abilities (owner KH1; on KH1's side: KH1's own)

| Id | Ability | While KH2 has Sora |
|---|---|---|
| 1 | High Jump | `Kh2`: KH2's High Jump LV1, higher only from Valor's form level |
| 2 | Mermaid Kick | `Kh1Own` (KH1's swimming) |
| 3 | Glide | `Kh2`: KH2's Glide LV1, higher only from Final's form level |
| 4 | Superglide | `Kh2`: KH2's Glide MAX |
| 5 | Treasure Magnet | `Kh1Own` |
| 6 | Combo Plus | `Kh2`: Combo Plus, one per copy |
| 7 | Air Combo Plus | `Kh2`: Air Combo Plus, one per copy |
| 8 | Critical Plus | `Kh1Own` (KH1's damage numbers) |
| 9 | Second Wind | `Hook` |
| 10 | Scan | `Kh1Own` |
| 11-15 | Sonic Blade, Ars Arcanum, Strike Raid, Ragnarok, Trinity Limit | `Kh1Own` (KH1's commands take him) |
| 16 | Cheer | `Kh1Own` |
| 17 | Vortex | `Hook` (was Flash Step) |
| 18 | Aerial Sweep | `Hook` (was KH2's Aerial Sweep) |
| 19 | Counterattack | `Hook` (was Counterguard) |
| 20 | Blitz | `Hook` (was Finishing Leap) |
| 21 | Guard | `Kh2`: Guard |
| 22 | Dodge Roll | `Kh2`: KH2's Dodge Roll LV1 (KH2 grows it with Limit Form, not on KH1's states here) |
| 23-28 | MP Haste, MP Rage, Second Chance, Berserk, Jackpot, Lucky Strike | `Kh1Own` |
| 29-52 | Donald's, Goofy's and the guests' abilities | `Kh1Own` |
| 53 | Slapshot | `Hook` (was KH2's Slapshot) |
| 54 | Sliding Dash | `Hook` (was Slide Dash) |
| 55 | Hurricane Blast | `Hook` (was Explosion) |
| 56 | Ripple Drive | `Hook` (was Vicinity Break) |
| 57 | Stun Impact | `Hook` (was Guard Break) |
| 58 | Gravity Break | `Hook` (was Aerial Finish) |
| 59 | Zantetsuken | `Kh1Own`. In Sora's list again since combo row selection m22 ("bring back zantetsuken"); a copy the mod took out before is given back once, off. Played by KH1's own 30% roll in the finisher walk (`docs/kh1-anim-slots.md` §7.z.1). |
| 60-64 | Tech Boost, Encounter Plus, Leaf Bracer, Evolution, EXP Zero | `Kh1Own` |
| 65 | Combo Master | `Kh2`: Combo Master |

The `Kh2` mappings are abilities that both games have with the same job. KH1's own jump, glide,
roll and guard keep KH1's behaviour at a move style KH1. With a move style at KH2, KH2's numbers
for the same ability apply: KH2's arc, KH2's glide, KH2's roll level.

### KH2's abilities (owner KH2; ours)

On KH2's side, each one is its own KH2 value (`Kh2(effect)`). All of them are in Sora's own list,
and use AP.

| Id | Ability | AP | While KH1's states have Sora |
|---|---|---|---|
| 66 | Quick Run | 2 | `Hook` (KH1's roll at Dodge Roll KH1) |
| 67 | Aerial Dodge | 3 | `Borrow` (a double jump at Jump KH1 too) |
| 68 | Aerial Recovery | 2 | `Hook` |
| 69 | Dodge Slash | 2 | `Kh1Combo` |
| 70 | Horizontal Slash | 2 | `Kh1Combo` |
| 71 | Negative Combo | 2 | `Kh1Combo` |
| 72 | Aerial Spiral | 2 | `Kh1Combo` |
| 73 | Upper Slash | 4 | `Kh1Combo` |
| 74 | Finishing Plus | 5 | `Kh1Combo` |
| 75 | Retaliating Slash | 3 | `Hook` |
| 76 | Aerial Dive | 2 | `Kh1Combo` |
| 77 | Magnet Burst | 3 | `Kh1Combo` |
| 78 | Flash Step | 2 | `Kh1Combo` |
| 79 | Aerial Sweep | 2 | `Kh1Combo` |
| 80 | Counterguard | 4 | `Hook` |
| 81 | Finishing Leap | 5 | `Kh1Combo` |
| 82 | Slapshot | 2 | `Kh1Combo` |
| 83 | Slide Dash | 2 | `Kh1Combo` |
| 84 | Explosion | 3 | `Kh1Combo` |
| 85 | Vicinity Break | 2 | `Kh1Combo` |
| 86 | Guard Break | 3 | `Kh1Combo` |
| 87 | Aerial Finish | 3 | `Kh1Combo` |

All of them are Sora's from the start (the tester, what-is-still-missing after m17: "make it all
vanilla, just give the player all the custom abilities we added"), off until equipped. Quick
Run is LV1, higher from Wisdom's form level; Aerial Dodge stays LV1 (KH2 grows it with Master
Form).

The AP is KH2's, read from the user's KH2 `03system.bin` item table (ability number → AP:
Flash Step 2, Aerial Sweep 2, Counterguard 4, Finishing Leap 5, Slapshot 2, Slide Dash 2,
Explosion 3, Vicinity Break 2, Guard Break 3, Aerial Finish 3).

Two KH2 abilities share their KH1 namesake's name: Aerial Sweep and Slapshot. Their help text
starts with "KH2's" so that they can be told apart in the menu.

## How it works

- **The table in the game** (`menu::ensure_table`). KH1 reads an ability's entry with
  `0x14028D760 (id)`: table + ((id & 0x7F) − 1) × 12, with no bound check. Its table pointer
  `0x142D37748` is pointed at our copy, which has 127 entries:
  - KH1's 65 entries are copied as they are;
  - each of ours is made from its template's entry (KH1's namesake for 78-87, so it sits next
    to it in the menu), with KH2's AP and our own name and help text.

  KH1 counts an equipped ability with `0x140282A70 (id, member)`, which scans the list for the
  id. The AP sums in `0x1402E6220` and `0x1402E62D0` walk the list's 48 bytes. Nothing in KH1
  indexes an array by ability id, so ids past 65 are only ever looked up in the table.
- **KH2's values** (`Kh1Sora::kh2`). Every equipped ability whose KH2 side is `Kh2(effect)`,
  KH1's or ours, sets that value in `Abilities`. Two copies of Combo Plus give 2. Growth levels
  take the highest: KH1's Glide gives LV1 (or Final's level), and Superglide gives MAX.
- **The hooks** (`Kh1Sora::kh1_hooks`, `kh2_hooks`). An ability that is on but has no meaning on
  the side that has Sora is logged ("abilities on with nothing on the other side yet: ...").
- **Progression is KH1's vanilla one** (the tester, what-is-still-missing after m17: "make it all
  vanilla, just give the player all the custom abilities we added, existing saves get updated,
  remove extra ap for now"). KH1 gives its own abilities on its own schedule (level-up tables,
  story rewards). The mod never holds one back, takes one out, renames it, keeps only one copy
  or removes one to make room. Gone: the level schedule (`MenuAbility::level`) and the boss gifts (`BOSS_GIFTS`),
  the per-level growth tables, `make_room`, `one_only` (spare High Jump / Glide / Superglide
  copies), Aerial Dodge taken back until Genie Jafar, the bonus AP.
- **Our KH2 abilities** (`Kh1Sora::grants`): every one of `MENU` in Sora's own list is his, off
  until equipped, with KH2's AP out of KH1's AP (no bonus). Several at once get one box ("New
  Abilities: n KH2 Abilities"). KH2's own Glide (88) only comes with the Glide style.
- **KH1 first** (`growth::RESERVE`, `yield_one`). KH1 appends a new ability at the first 0 of the
  48 bytes, so a full list would lose it. Ours always leave 2 places free; when KH1's own fill the
  list past that, one of ours gives its place back (the last in `MENU` order that is off, else
  on) and returns when there's room. Assumption: KH1 never gives more than 2 abilities within one
  second (the check runs once a second). On a late-game list (KH1's own ~37 or more) not all 22
  of ours fit: see "Not yet".
- **Growth only from Drive Form levels** (the tester, after m16, answer 6 (b); KH2's
  `0x1403E8380`, Checked (decomp)). An ability he has is LV1. Valor's, Wisdom's and Final's
  levels raise High Jump, Quick Run and Glide to LV1 / 2 / 3 at form LV3 / 5 / 7
  (`Kh1Sora::form_growth`). Sora's level and bosses give none. KH1's own High Jump and Glide count
  as KH2's LV1 once KH1 has given them (Assumption taken: the tester's question 3 after m17
  wasn't answered); a form's level no longer puts KH1's High Jump or Glide in his list (KH1's
  list stays vanilla). KH1's Superglide is KH2's Glide MAX, and KH2's own Glide (88) is MAX past
  Chernabog, where KH1 gives Superglide. Dodge Roll and Aerial Dodge stay LV1: in KH2 they grow
  with Limit and Master, which aren't on KH1's states here. The debug menu's growth rows still
  override all of this.
- **The fmlv rows' abilities** (KH2's own form level-up rule, not the mod's progression; kept):
  Combo Plus, Air Combo Plus, MP Rage, MP Haste (Wisdom LV6, KH1's own MP Haste, t714; it becomes
  MP Charge's MP Haste once a KH2 MP system is a Config setting), as KH1's own, off, with a box.
  With Sora's list full, one of ours gives its place. They no longer add AP (t715's AP is gone
  with the bonus). The Auto forms and Form Boost have none yet. Details in
  `docs/kh1-drive-forms-native.md` §9. `abilities = kh1` only.
- **Growth boxes.** KH1's High Jump keeps KH1's name in the menu. When KH2's level for it, Quick
  Run's or KH2 Glide's grows (from a form's level), an "Ability Up" box says so.
- **No bonus AP.** The `bonus_ap` ini key is gone (an old ini's line is ignored); Sora has
  KH1's own AP only.

## The save

- **Sora's abilities** stay in KH1's own lists: Sora's 48 bytes at save `+0x44`, and the shared
  10 bytes at `+0x599`. KH1's menu shows and equips our ids there like its own. KH1's lists can't
  grow: the menu walks 48, and the next party record starts at `+0x74`.
- **The layout mark**: the u16 at save `+0x48A` is `0x4B02` (`growth::LAYOUT_SPLIT`) once the save
  has been moved over, or once it was found with nothing to move. These bytes are free:
  - they are among the four spare bytes `+0x488..+0x48B` between the ten party records and
    `+0x48C`;
  - `docs/kh1-equipment.md` section 5 scanned every `.text` instruction for displacements
    `0x480..0x48B` and found none after a load of the save pointer;
  - this work scanned again for `0x489` / `0x48A` / `0x48B`, as a displacement, as an immediate,
    and as the absolute addresses `0x142DEA1E9..0x142DEA1EB`, and found no hits;
  - Valor's Keyblade uses `+0x488`, and `+0x489` is left for its marker.

  The whole block is the save data, so the mark goes with the save. Confidence is medium-high: a
  static scan can't see an access through a pointer computed elsewhere.

## Old saves

The layout mark (save `+0x48A`) is `0x4B03` (`growth::LAYOUT_VANILLA`) once a save is on the
vanilla progression. A save without it is moved over once, the first time the game sees it with
`abilities = kh1`:

1. **The split** (`growth::migrate`, only for a save from before testing s261, holding the old ids
   66-77): each KH1 modifier Sora has brings its KH2 ability, on if KH1's was on (KH1's then off).
   With no room beside KH1's own it comes later; it never takes a KH1 ability's place.
2. **Vanilla back** (`growth::restore_vanilla`, only for a save the mod had changed: the split's
   mark, the old ids, or any of ours in it). Everything `kh1-kh2-removed.txt` (next to the game)
   noted as taken out, up to Sora's current level, comes back off, one per copy taken out: KH1's
   abilities the older builds removed for room, their spare copies, Zantetsuken, and the shared
   spares (High Jump, Glide, Superglide). Superglide comes back if Chernabog is beaten and KH2's own
   Glide isn't standing for it. With the list full, ours give their places back. Every change is
   in the log ("abilities: ..."). Then the rest of ours are given where there's room.

A save the mod never changed (a new game) just gets the mark.

**What can't be undone.** The file is the install's, not the save's: with several saves, each
gets back what was noted up to its own level (Assumption: in practice one main save). KH1 grants
an older build held back without noting it (the KH1 level-up copies of abilities the mod had
given early) aren't in the file and don't come back; neither do the old build's early KH1
copies go (they're learned KH1 abilities). The equipped state of what comes back is off. The
file is no longer written to: nothing is taken out any more.

## Not yet

- **Room for all of ours.** KH1's list is 48 bytes and the next party record starts at `+0x74`, so
  on a late-game list not every KH2 ability fits beside KH1's own. A list of our own (22 bytes
  for the 22, in free save bytes, read by KH1's Abilities menu after KH1's 48: the menu's list
  walk, its AP sums `0x1402E6220` / `0x1402E62D0` and the count `0x140282A70` would all need to
  take it) is the native fix: asked first.

- The `Hook`s: KH1's modifiers while KH2 has Sora (KH2 Sora with KH1's Hurricane Blast), and
  KH2's Quick Run, Aerial Recovery, Retaliating Slash and Counterguard while KH1's states have
  him.
- `Kh1Combo` is `win/combat.rs`'s work, done by another helper. This side only fills
  `Abilities`.
- The `Kh1Own` rows on KH2's side are not checked one by one in the game.
- Nothing here has been tried in the game.
