# Abilities: KH1's ids and KH2's ids (testing branch)

Code: `crates/kh1-hook/src/growth.rs` (the table, learning, migration), `crates/kh1-hook/src/win/menu.rs`
(KH1's ability table, the save, the boxes), `crates/kh1-hook/src/win/kh2list.rs` (the KH2 list in KH1's
Abilities menu). Ini: `abilities = kh1`.

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
| 4 | Superglide | `Kh2`: KH2's Glide LV1, higher only from Final's form level (after m18, answer 3 (b)) |
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

On KH2's side, each one is its own KH2 value (`Kh2(effect)`). All of them are in the KH2 list (its own
list, a row "KH2" after Sora's in KH1's Abilities menu, see "The KH2 list"), and use Sora's AP.

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

  KH1 counts an equipped ability with `0x140282A70 (id, member)` (Steam `0x140284BE0`), which
  scans the list for the exact byte (equipped copies only). The AP sums (Steam `0x1402E8660` AP
  left, `0x1402E8710` AP used) walk the list's 48 bytes. Nothing in KH1 indexes an array by
  ability id, so ids past 65 are only ever looked up in the table.
- **KH2's values** (`Kh1Sora::kh2`). Every equipped ability whose KH2 side is `Kh2(effect)`,
  KH1's or ours, sets that value in `Abilities`. Two copies of Combo Plus give 2. Growth levels
  take the highest: KH1's Glide and Superglide each give LV1 (or Final's level).
- **The hooks** (`Kh1Sora::kh1_hooks`, `kh2_hooks`). An ability that is on but has no meaning on
  the side that has Sora is logged ("abilities on with nothing on the other side yet: ...").
- **Progression is KH1's vanilla one** (the tester, what-is-still-missing after m17: "make it all
  vanilla, just give the player all the custom abilities we added, existing saves get updated,
  remove extra ap for now"). KH1 gives its own abilities on its own schedule (level-up tables,
  story rewards). The mod never holds one back, takes one out, renames it, keeps only one copy
  or removes one to make room. Gone: the level schedule (`MenuAbility::level`) and the boss gifts (`BOSS_GIFTS`),
  the per-level growth tables, `make_room`, `one_only` (spare High Jump / Glide / Superglide
  copies), Aerial Dodge taken back until Genie Jafar, the bonus AP.
- **Our KH2 abilities** (`Kh1Sora::grants`): every one of `MENU` (but KH2's own Glide) is his, in
  the KH2 list, off until equipped, with KH2's AP out of Sora's AP (no bonus). Several at once get
  one box ("New Abilities: n KH2 Abilities"). KH2's own Glide (88) only comes with the Glide style
  and stays in the shared list.
- **KH1's 48 are KH1's alone** (the tester, after m18, answer 1). t718's `RESERVE` / `yield_one`
  (ours leaving places free in Sora's list and giving them back) are gone: ours are never in it.
- **Growth only from Drive Form levels** (the tester, after m16, answer 6 (b); KH2's
  `0x1403E8380`, Checked (decomp)). An ability he has is LV1. Valor's, Wisdom's and Final's
  levels raise High Jump, Quick Run and Glide to LV1 / 2 / 3 at form LV3 / 5 / 7
  (`Kh1Sora::form_growth`). Sora's level and bosses give none. KH1's own High Jump and Glide count
  as KH2's LV1 once KH1 has given them (Assumption taken: the tester's question 3 after m17
  wasn't answered); a form's level no longer puts KH1's High Jump or Glide in his list (KH1's
  list stays vanilla). KH1's Superglide is KH2's Glide LV1 too, and KH2's own Glide (88) is LV1
  past Chernabog as before it: only Final's level raises Glide (the tester, what-is-still-missing
  after m18, answer 3 (b): "treat it as LV1 so only Final raises Glide"; until t721 both were
  MAX). Dodge Roll and Aerial Dodge stay LV1: in KH2 they grow
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

## The KH2 list

The tester, what-is-still-missing after m18, answer 1: "give kh2's its own ability list right next
to sora, should be titled "KH2"". Steam only (`win/kh2list.rs`).

- **Storage.** 48 bytes in KH1's config block (save sub-block 5; its pointer Steam `0x142E1F4D8`,
  Epic `0x142E1FE70`) at `+0x3C0..+0x3EF` (`growth::KH2_LIST_OFF`), in KH1's list format (id, bit 7
  = not equipped, 0 ends it). The config block is part of the save slot (saved and loaded with it,
  zeroed at a new game, `docs/kh1-config-menu.md` 1.1). Checked (decomp, Steam): every access
  through `0x142E1F4D8` is below `+0x78` (the option ints, the journal bit arrays at `+0x30..+0x74`,
  indexed by small bit numbers). Assumption: no access through a pointer computed elsewhere (a
  static read can't see one); the offset is far past `+0x78` to keep clear of any bit array.
- **KH1's own Abilities menu shows it** as a row "KH2" right after Sora's (Checked (decomp + exe),
  Steam):
  - The page: init `0x140311850` (the pause menu's page-init table `0x14052ADA0`, entry 2), its
    main window `0x140310EF0`, the member picker `0x140310B60`, the list window `0x140310A90`
    (`0x140311C20` for a member's list, `0x1403120F0` for the shared one), the header
    `0x1403108E0` (name, AP left / max), the help line `0x140310AB0`, the sort window
    `0x140310400`, the equip window `0x140310650`. Their definitions (`.data`, first qword the
    update function): `0x1405490F0`, `0x1405490C0`, `0x140549090`, `0x140548FA0`, `0x140548F40`,
    `0x140549030`, `0x140549060`.
  - The picker's rows are the party slots, `save + 0x48E + row` (count `0x142E941D0`, set by
    `0x1402E7EC0`: slots `+0x48E..+0x490` until 0xFF, one more for a guest in `+0x491`), then the
    shared list. A member row's list is `save + 4 + member * 0x74 + 0x40` (48 bytes), counted at
    page init into `0x142E9CBC8[row]` (4 ints); choosing it sets the row `0x142E9CB7C`, the member
    `0x142E9CB80` and the record `0x142E9CB88`, which the equip (toggle bit 7) and sort windows use.
  - Each of those eight update pointers is wrapped. For the length of KH1's own call the page sees
    the party with our member (10; the save's records are 0..9) after Sora, and the KH2 list where
    KH1's record arithmetic puts member 10's list, `save + 0x4CC`; after the call the list is
    copied back to its own place and the bytes that were there (KH1's item counts
    `save + 0x499 + item`, items 0x33..0x62, which the page doesn't read) are put back, with the
    party, the count, and, with a guest (5 rows), the 5th row's count, which lands on
    `0x142E9CBD8` (another page's window pointer, set by that page's own init) and is kept between
    calls. The game's thread runs nothing else in between, so outside the page's own calls the
    save is KH1's exactly.
  - KH1's code then counts, draws, sorts, equips and refuses (AP) the KH2 list as a member's, with
    its own windows and sounds.
  - Re-made (RIP-relative, replaced outright): the member name `0x1402E86F0` (member 10 is "KH2";
    the others KH1's name text `0x1402D1510 (id)` by the id table `0x1405452B8`), max AP
    `0x1402E8560` (member 10 reads Sora's, stats copy `0x142E942C4 + member * 0x100`), the picker's
    cursor clamp `0x1403119D0` (from the pause menu; clamps to the page's count with the KH2 row).
  - KH1's party-stats refresh `0x1402E7EC0` (hooked) runs with the real party and items under it
    when the page calls it, then the page's count is set again.
- **AP.** KH1's AP left `0x1402E8660 (member)` and used `0x1402E8710 (member)` are hooked: for Sora
  (and member 10) they add the AP of the KH2 list's equipped abilities (KH1's ability table `+0`,
  our entries hold KH2's AP). So both lists share Sora's AP in every AP KH1 shows or checks
  (Abilities, Status, the Equipment page's accessory check).
- **What reads it.** `growth::Kh1Sora::kh2` (the KH2 values, hooks, grants; a copy of ours still in
  Sora's own list counts too, so a save not moved over keeps working), the debug menu's Abilities
  page (adds / takes ours there).
- **Moving a save over** (`growth::move_to_kh2_list`, once, mark `0x4B04` = `growth::LAYOUT_KH2_LIST`
  in the u16 at save `+0x48A`): the KH2 list is cleared first (its bytes were nobody's), every one
  of ours in Sora's 48 moves to it as it was (on or off), KH1's keep their order with the gaps
  closed. An older save's split (`migrate`) puts its KH2 abilities in the KH2 list, and ours move
  out before `restore_vanilla` gives KH1's back, so they have the room. Log: "abilities: ... moved
  to the KH2 list", "the KH2 abilities have their own list now".
- **Other builds** (Epic), or a check that fails at install ("KH2 list: not installed" in the log):
  ours stay in Sora's own list as in t718, given only while it has more than 2 places free
  (`FALLBACK_RESERVE`), never in a KH1 ability's place.
- Log lines: "KH2 list: installed", "KH2 list: Abilities menu shows a KH2 row at row 1 (party
  [..] -> [..])", "KH1 Sora: ... KH2 list [..]", "Sora is given X (..., KH2 list)".

## The save

- **Sora's abilities** stay in KH1's own lists: Sora's 48 bytes at save `+0x44`, and the shared
  10 bytes at `+0x599`; ours in the KH2 list (above). KH1's lists can't grow: the menu walks 48,
  and the next party record starts at `+0x74`.
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

- The KH2 list has not been tried in the game yet (layout of a sixth picker row with a guest,
  the help line's text for the KH2 row).

- The `Hook`s: KH1's modifiers while KH2 has Sora (KH2 Sora with KH1's Hurricane Blast), and
  KH2's Quick Run, Aerial Recovery, Retaliating Slash and Counterguard while KH1's states have
  him.
- `Kh1Combo` is `win/combat.rs`'s work, done by another helper. This side only fills
  `Abilities`.
- The `Kh1Own` rows on KH2's side are not checked one by one in the game.
- Nothing here has been tried in the game.
