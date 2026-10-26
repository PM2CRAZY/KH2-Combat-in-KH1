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
| 1 | High Jump | `Kh2`: KH2's High Jump at Sora's growth level (LV2 at 30, LV3 at 44, MAX at 58) |
| 2 | Mermaid Kick | `Kh1Own` (KH1's swimming) |
| 3 | Glide | `Kh2`: KH2's Glide, LV2 at 40, LV3 at 52 |
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
| 22 | Dodge Roll | `Kh2`: KH2's Dodge Roll at Sora's growth level (LV2 at 19, LV3 at 37, MAX at 60) |
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

| Id | Ability | AP | Learned | While KH1's states have Sora |
|---|---|---|---|---|
| 66 | Quick Run (LV by Sora's level: 12 / 20 / 32 / 45) | 2 | level 12 | `Hook` (KH1's roll at Dodge Roll KH1) |
| 67 | Aerial Dodge (LV2 30, LV3 42, MAX 54) | 3 | Genie Jafar | `Borrow` (a double jump at Jump KH1 too) |
| 68 | Aerial Recovery | 2 | level 4 | `Hook` |
| 69 | Dodge Slash | 2 | level 10 | `Kh1Combo` |
| 70 | Horizontal Slash | 2 | level 16 | `Kh1Combo` |
| 71 | Negative Combo | 2 | level 20 | `Kh1Combo` |
| 72 | Aerial Spiral | 2 | level 25 | `Kh1Combo` |
| 73 | Upper Slash | 4 | Sabor | `Kh1Combo` |
| 74 | Finishing Plus | 5 | level 34 | `Kh1Combo` |
| 75 | Retaliating Slash | 3 | level 43 | `Hook` |
| 76 | Aerial Dive | 2 | Cerberus | `Kh1Combo` |
| 77 | Magnet Burst | 3 | Captain Hook | `Kh1Combo` |
| 78 | Flash Step | 2 | Sabor, or with KH1's Vortex | `Kh1Combo` |
| 79 | Aerial Sweep | 2 | level 28, Clayton, or with KH1's Aerial Sweep | `Kh1Combo` |
| 80 | Counterguard | 4 | level 22, or with KH1's Counterattack | `Hook` |
| 81 | Finishing Leap | 5 | level 48, or with KH1's Blitz | `Kh1Combo` |
| 82 | Slapshot | 2 | with KH1's Slapshot | `Kh1Combo` |
| 83 | Slide Dash | 2 | level 30, or with KH1's Sliding Dash | `Kh1Combo` |
| 84 | Explosion | 3 | Dragon Maleficent, or with KH1's Hurricane Blast | `Kh1Combo` |
| 85 | Vicinity Break | 2 | level 36, or with KH1's Ripple Drive | `Kh1Combo` |
| 86 | Guard Break | 3 | Pot Centipede, or with KH1's Stun Impact | `Kh1Combo` |
| 87 | Aerial Finish | 3 | Jafar, or with KH1's Gravity Break | `Kh1Combo` |

The learn points for 78-87 are the same as the ones each had while it was a KH1 id. The mod's
levels and bosses gave that KH1 id, and so did KH1's own level-ups. Now the mod's levels and
bosses give the KH2 id, and KH1's own level-ups still give KH1's ability. When KH1's ability is
learned, its KH2 namesake comes with it (`MenuAbility::with`). Slapshot never had a gift of the
mod's, so KH1's Slapshot is its only learn point.

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
  take the highest: KH1's Glide gives LV3 at 52, and Superglide gives MAX.
- **The hooks** (`Kh1Sora::kh1_hooks`, `kh2_hooks`). An ability that is on but has no meaning on
  the side that has Sora is logged ("abilities on with nothing on the other side yet: ...").
- **Learning** (`Kh1Sora::grants`). One of ours is learned at its level, when its boss is beaten
  (by the world's story progress, `BOSS_GIFTS`), or with its KH1 namesake. It comes off, with a
  "New Ability" box. When Sora's list is full, a KH1 ability that's off makes room (`make_room`,
  as before). KH1's combo modifiers are never given up, because KH1's own combo uses them.
- **From Drive Form levels** (the tester, what-is-still-missing after m15, answer 6 (c); KH2's
  `0x1403E8380`, Checked (decomp)). A form's level gives base Sora its growth one under the
  form's (Valor High Jump, Wisdom Quick Run, Final Glide: LV1 / 2 / 3 at form LV3 / 5 / 7,
  `Kh1Sora::form_growth`); the higher of that and his own level's counts, and without the
  ability it is learned (High Jump in the shared list, on; Quick Run off, its AP in the bonus).
  The fmlv rows' abilities come as KH1's own: Combo Plus, Air Combo Plus, MP Rage (off, "New
  Ability" box); the Auto forms, MP Haste and Form Boost have none yet. Details in
  `docs/kh1-drive-forms-native.md` §9. `abilities = kh1` only.
- **Growth boxes.** KH1's High Jump, Glide and Dodge Roll keep KH1's names in the menu. When KH2's
  level for them grows, an "Ability Up" box still says so ("High Jump LV2"). This is KH2's side
  only.
- **Bonus AP** (`bonus_ap`). Each KH2 ability Sora has earned (by level, boss or namesake) adds its
  AP to his max AP, capped at KH1's 100, as before.

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

## Old saves (`growth::migrate`)

A save is from before the split when it has no mark and holds any of the old ids 66-77. It is
moved over once, the first time the game sees it with `abilities = kh1`:

- **KH1's modifiers.** For each KH1 modifier Sora has (Hurricane Blast, Vortex, ...), its KH2
  ability (Explosion, Flash Step, ...) is added. It is on if KH1's was on, so KH2's combo has the
  same moves as before. KH1's copies are turned off, which frees their KH1 AP; KH2's AP is
  covered by the bonus. KH1's abilities stay learned, and the user can turn them back on for
  KH1's own combo.
- **Superglide.** The old build turned Superglide into Glide or took it out. It comes back if
  Chernabog is beaten (End of the World progress ≥ `0x33`), on if Glide is.
- Each change goes to the log ("abilities: ...") and to `kh1-kh2-removed.txt`.

**What isn't undone.** The old build gave some KH1 abilities early (Vortex for Sabor, Sliding
Dash at 30, ...), and took out KH1's own later copies of them. Those early KH1 copies stay: they
are learned KH1 abilities now. KH1's own level-ups may later give a second copy. Zantetsuken is no
longer taken out (m22); one the mod took out earlier (its line "Zantetsuken taken out" in
`kh1-kh2-removed.txt`) is given back once, off, making room if the list is full.

A save without the old ids (a new game, or a plain KH1 save) just gets the mark. Its KH1
modifiers stay as they are, and their KH2 namesakes come off.

## Not yet

- The `Hook`s: KH1's modifiers while KH2 has Sora (KH2 Sora with KH1's Hurricane Blast), and
  KH2's Quick Run, Aerial Recovery, Retaliating Slash and Counterguard while KH1's states have
  him.
- `Kh1Combo` is `win/combat.rs`'s work, done by another helper. This side only fills
  `Abilities`.
- The `Kh1Own` rows on KH2's side are not checked one by one in the game.
- Nothing here has been tried in the game.
