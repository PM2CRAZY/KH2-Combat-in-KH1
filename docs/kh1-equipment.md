# KH1 equipment, inventory and the Equipment page (for Valor's Keyblade)

Research notes (Ghidra and disassembly, Epic KH1 Final Mix PC, image base `0x140000000`; Steam
addresses from `/opt/re/steam`'s matcher) for Valor Form's second Keyblade: a row in KH1's
Equipment page for it, filled with a Keyblade Sora owns and isn't holding; with none, no Valor.
Read statically; nothing here has been tried in the game yet. Read-only helpers:
`crates/kh1-hook/src/win/equip.rs` (not in `mod.rs` yet).

Confidence: **high** = read in the code, every step seen; **medium** = read in the code with one
step inferred (a name, a purpose); **low** = from memory or community lists, not in the exe.

`save` is `*0x1428695A0` (Steam `0x142868BA0`), with KH1's static block `0x142DE9D60` (Steam
`0x142DE9360`) behind it (`menu::save_block`).

## 1. Party records and Sora's weapon

The save starts with **ten party records of `0x74` bytes** (member 0 = Sora, 1 Donald, 2 Goofy,
then the world guests). For record `R = save + member × 0x74` (high, `0x1402A2FA0`,
`0x1402E6140`, `0x1402E6330`, `0x140282CF0`):

| Offset | What |
|---|---|
| `R+0x04` | level (compared with 100 by the EXP code `0x1402826DD`) (medium) |
| `R+0x05 / +0x06` | HP / max HP (high) |
| `R+0x07 / +0x08` | MP / max MP (high) |
| `R+0x09` | AP (high: the stats block's `+0xE4`) |
| `R+0x0A / +0x0B` | Strength / Defense (high) |
| `R+0x0C .. +0x1B` | 16 element resistances, percent (high) |
| `R+0x1C` | accessory slot count; `R+0x1D ..` the accessories (item ids, 0 = empty) (high) |
| `R+0x25` | item slot count; `R+0x26 ..` the items carried (high) |
| `R+0x36` | **the weapon** (item id, 0 = none) (high) |
| `R+0x3C` | a u16 the stats copy into `+0xBE`, capped at max MP × 30 (high; meaning unknown) |
| `R+0x40` | EXP, u32 (medium) |
| `R+0x44 .. +0x73` | the ability list, 48 bytes (high; matches `save + 0x44` for Sora) |

So **Sora's Keyblade is `save + 0x36`** (high; `pose.rs` already reads it).

`save + 0x48E .. 0x490` are the three party slots (member ids, `0xFF` empty), `+0x491` a fourth
(guest) slot; `+0x48C / +0x48D` are bytes set by `0x1402835B0 / 0x1402835A0` (`+0x48D` picks
Sora's level-up table: medium); `+0x492 + spell` the spell levels.

### The item table: what is a Keyblade (high, except the names)

KH1's item table is in `btltbl.bin` (loaded by `0x14028D840`): the pointer at `0x142D23738`
(Steam `0x142D22D38`), 255 records of `0x14` bytes, the record of item `id` at
`table + (id − 1) × 0x14` (`0x14028D7E0 (id)`):

| Offset | What |
|---|---|
| `+0x00` | u16: the name's word number (name = `0x14028D820 (word)`, word handles at `0x142D23998`) |
| `+0x02` | **kind**: 0 an item, 1 an accessory, `2 + member` that member's weapon (2..11; table `0x140545FC0`), 12 and 13 other kinds (`0x140546010`: 0, 12, 13 count as "items" for one test) |
| `+0x03` | a byte (`0x1402E6620`) |
| `+0x04` | u16 flags; bit 0: used on the whole party (`0x1402E6750`) |
| `+0x06` | i16: for a weapon, its row (1-based) in its member's weapon table; for an accessory, its row in the accessory stats (`0x14028D780`) |
| `+0x08` | u16: the price (`0x1402E64B0`, the shop's cost × count) (medium) |
| `+0x0A`, `+0x10` | u16s read by `0x1402E65E0 / 0x1402E6600` |
| `+0x0C` | the help text's handle (`0x1402E6540`) |

- **A Sora weapon is kind 2** (`0x1402E6BC0 (id, member)` checks `kind == 0x140545FC0[member]`,
  and `0x140545FC0` is `2, 3, .. 11`) (high).
- Not every Sora weapon is a Keyblade: KH1 locks Sora's weapon row while he holds
  **`0x52, 0x53, 0x54, 0x55`** (`0x1402E6C60`, list at `0x140546020`), and `0x140304BC0` treats
  `0x51..0x55` as his starting weapons. These are the Dream Sword / Shield / Rod and the wooden
  sword (medium: the ids high, the names from `pose.rs`'s in-game checks and the community).
- So **Keyblade = kind 2 and not `0x52..=0x55`** (high as a rule; `equip::is_keyblade`).
- The ids by name (low, community lists, not checked in the data): `0x51` Kingdom Key, `0x56`
  Jungle King, `0x57` Three Wishes, `0x58` Fairy Harp, `0x59` Pumpkinhead, `0x5A` Crabclaw,
  `0x5B` Divine Rose, `0x5C` Spellbinder, `0x5D` Olympia, `0x5E` Lionheart, `0x5F` Metal
  Chocobo, `0x60` Oathkeeper, `0x61` Oblivion, `0x62` Lady Luck, `0x63` Wishing Star, `0x64`
  Ultima Weapon, `0x65` Diamond Dust, `0x66` One-Winged Angel. Donald's first staff is `0x67` and
  Goofy's first shield `0x77` (high: `0x140304BC0` names them as their starting weapons). The
  add-item code `0x140282CF0` treats `0x60`, `0x65` and `0x66` specially (calls `0x1400D3980`
  with `id − 0x57`: likely trophies) (medium). Use the kind test, not the list.

### Weapon stats (high)

- `0x140284960 (member, id)` / `0x140284880 (member)` (equipped): the member's weapon table is at
  `0x142D23740 + 0x140528D60[member]` (an offset into `btltbl.bin`; Sora's `0x94F8`), rows of
  `0x58` bytes, row `item[+6] − 1`.
- A row's **`+0x30` (i8) adds to Strength, `+0x38` (i8) to max MP**. The rest of the row is the
  weapon's own data (its model is loaded from it by `0x140284A60`; the weapon object keeps the
  row at `+0x20`, `0x140284D40`).
- The stats of a member, `0x1402A2FA0 (member, stats)`: base from the record, + the weapon row's
  Strength / MP, + every accessory (`0x14028D780`: `0x16`-byte rows at `*0x142D37740`: HP, MP,
  Strength, Defense, AP, then 16 resistance percents), then the caps (Strength, Defense, AP ≤ 100).
- Where it is applied: the pause menu's copy for the ten members at `0x142E94BC0 + member × 0x100`
  is refilled by `0x1402E5A80` (called after each equipment change, and as the menu opens); the
  party actors' stats blocks (`actor + 0x6C`) by `0x1402A2CC0` (each of the three actors at
  `0x142D37C80`, member from `actor+0x130`'s `+0x4C`, HP / MP kept), called from `0x1402D3040`
  (medium: by its place in the menu code, the pause menu's exit).
- What-ifs for the compare window: `0x1402E69A0 (member, weapon)` = Strength with that weapon
  (writes `+0x36`, fills a scratch stats block, puts it back); `0x1402E5DA0` and its siblings do
  the same for an accessory slot.

### Live stats and Valor's second Keyblade (high, read statically; s220)

- A party actor's stats block (`actor + 0x6C`, filled by `0x1402A2FA0`): `+0x3C` HP, `+0x40`
  max HP, `+0x44` MP (bars), `+0x48` max MP, `+0x4C` Strength, `+0x50` Defense, `+0xBE` the MP
  gauge (u16, at most max MP × 30), `+0xE4` AP, `+0xC8` the save record it came from.
- `0x1402A2CC0` (the party refresh) runs the stats function into each of the three actors'
  blocks: HP and MP are taken from the save record (`R+0x05` / `+0x07`, then written back), so
  it can't be run mid-fight to refresh one number.
- Level-up bonuses are applied in place too (`0x1402B34C0 (member, kind)`: max HP +3, max MP +1,
  AP +2, Strength +2, Defense +2 on both the record and the live block), so the live block is
  not always one stats-function call old.
- Valor's second Keyblade (`kh1-hook/src/win/valorstats.rs`): while Sora is Valor his live
  Strength and max MP are set each KH2 step to what the stats function gives for his own
  equipment (run into a scratch block, unhooked) plus the Valor Keyblade's row `+0x30` / `+0x38`
  (Strength capped at 100, max MP at least 0), with MP and the gauge clamped as KH1 clamps them;
  once out of Valor they are set back to the stats function's figures. The hooked stats function
  adds the same for Sora while he is Valor, so the pause menu's pages show it. Current MP is not
  raised by a bigger max (KH1's equipping doesn't either).

## 2. The inventory (high)

- **One count per item id at `save + 0x499 + id`** (ids 1..255; capped at 99). `0x140282CF0
  (id, member)` adds one (a kind-0 item into the member's item slots first), `0x140282B70`
  takes one. Weapons and accessories are counted there like everything else.
- **The count is everything owned, equipped copies included.** Equipping changes no count: the
  weapon list's confirm (`0x140303090`) and the accessory list's (`0x140303810`) only write the
  record's byte. KH1's "free" count `0x1402E6330 (id)` is `count − (members holding it as weapon
  + accessory slots holding it)` over the ten members; `0x1402E63C0 (id)` is that number
  equipped.
- So: **Sora owns Keyblade `k` when `save[0x499 + k] > 0`; it is free for Valor when that count
  is more than the copies equipped** (in practice: owned and not `save[0x36]`).
  (`equip::owned_keyblades`, `equip::valor_candidates`.)
- KH1's weapon list for a member (`0x140304E50`) is every id with a count and the member's weapon
  kind, sorted (`0x14030C130`), the equipped one included; its counts are shown when above 1.

## 3. Changing the weapon (high)

- **`0x1402845A0 (id, done)`**: Sora's (or any member's, by the item's kind) weapon in the field:
  **writes the record's `+0x36`** (`save + kind × 0x74 − 0xB2`), sets `0x142E9AA30` to the member,
  frees the old weapon object (actor `+0x14C`), loads the new model (`0x140284A60`; three load
  slots at `0x14296BF40`), then builds the object (`0x140284D40`: nine at `0x14296BBE0`,
  in-use bits `0x14296BFD8`) and calls `done`. The pause menu calls it from `0x140302BA0` (the
  preview) and `0x140302AF0` (closing the list).
- `0x140284BA0 (id)`: the same write without the model (from an event-script op,
  `0x1401AE210`).
- `0x140304BC0`: after a change, sets the u32 `save + 0x840` to 1 unless every member still holds
  his starting weapon (Sora `0x51..0x55`, Donald `0x67`, Goofy `0x77`) (high; what the flag is for:
  unknown).
- **To notice a change**: the menu's commit is a plain byte write, so poll `save + 0x36` each frame
  (as `pose.rs` does), or wrap the weapon list window (below). `0x1402845A0` runs for the live
  preview too (it writes `+0x36` while the cursor rests on a weapon), so it is not a commit.

## 4. The Equipment page

The pause menu's pages are opened from the table at `0x14052B7C0` (8 qwords: Items `0x14030C270`,
**Equipment `0x140304C30`**, `0x14030F410`, Customize `0x140301CF0`, `0x140311010`, ...; their
labels sysmsg `0x69..0x70`) (medium for the names; Steam: not mapped, a data gap).

Windows are made from 0x30-byte definitions (`+0` update function, `+0x10` type, `+0x14 / +0x16`
x / y, `+0x18` width, `+0x28` visible rows) by `0x1402E3660 (def, ..)`; a live window keeps its
row count at `+0xF8`, visible rows `+0xFA`, the first visible row and the cursor (×16) at
`+0xFC / +0xFE`, its height (×16) at `+0xE6`, its child at `+0x20`; the confirm test is
`0x1402EDF00 (window)`, rows are `[0x140546050] + [0x140546058]` apart (26 + 15). (high)

| Window (def) | Update | Steam (def / update) | What |
|---|---|---|---|
| `0x140549200` | `0x140304360` | `0x140548520` / `0x1403067A0` | the page; rebuilds its windows by state `0x142E9AA40` (0 members, 1 slots, 2 a list open, 3 changed) and draws the header (member, current item `0x142E9AA5C`, new item `0x142E9AA60`) |
| `0x1405491D0` | `0x140304100` | `0x1405484F0` / `0x140306540` | the members (party slots `save + 0x48E..0x491`, count `0x142E94BB0`); confirm sets slot `0x142E9AA44`, member `0x142E9AA48`, record pointer `0x142E9AA50` (= `R + 4`) |
| `0x1405491A0` | **`0x140303BE0`** | `0x1405484C0` / `0x140306020` | **the slots**: row 0 the weapon, rows 1.. the accessories |
| `0x140549080` | `0x140303B30` | `0x1405483A0` / `0x140305F70` | the slots' help line |
| `0x140549140` | **`0x140303090`** | `0x140548460` / `0x1403054D0` | **the weapon list** |
| `0x140549170` | `0x140303810` | `0x140548490` / `0x140305C50` | the accessory list |
| `0x140548FC0` | `0x140302C90` | `0x1405482E0` / `0x1403050D0` | the weapon compare box (Strength, MP) |
| `0x140548F90` | `0x140302BA0` | `0x1405482B0` / `0x140304FE0` | the live preview of the weapon in Sora's hand |

### The slots window `0x140303BE0` (high)

- Rows: set by others when the window is made, `record[+0x18] + 1` (accessory count + 1) at
  **`0x1403041A5`** (members' cursor moves; `0F B6 4C 11 1C FF C1`), **`0x1403043EF`** (state 1 / 3)
  and **`0x140304850`** (state 0) (both `0F B6 48 18 FF C1`), visible rows capped at 8 and the
  height from them.
- At entry: `r12 = save + member × 0x74` (`0x140303C15`: `4C 6B E6 74 4C 03 E2`); the row under
  the cursor is `0x142E9AA68 + 0x142E9AA6C` (from `+0xFC / +0xFE`).
- Confirm on **row 0** (`0x140303CA4`): for Sora, refused (sound 4) while `0x1402E6C60` says his
  weapon is locked; else `0x142E9AA5C` = the slot's chosen weapon `0x142E9AA90[slot]`, the list is
  filled (`0x140304E50`), and the weapon list, compare box and (party slots 0..2 only) the preview
  are opened.
- Confirm on **row n ≥ 1**: `0x142E9AA4C = n − 1` (`0x140303E8A`), `0x142E9AA5C = [r12 + n + 0x1C]`
  (`0x140303E96`), the accessory list is filled (`0x140304D40`) and opened.
- Drawing (`0x140303F87..`): each visible row `n` is `0x140304F30 (window, state, 0, y, 0xA6,
  item)` (call at `0x14030402E`) with item `0x142E9AA90[slot]` for row 0, else
  **`[r12 + n + 0x1C]`** (`0x140304001`: `41 0F B6 4C 1C 1C`).
- `0x140304F30` draws the row's name (**`0x1402E6570 (item)`, called at `0x140304F74`**; sysmsg
  `0xA7` for none) and a bullet (`0x1402D3AB0`); it draws the list rows too.
- The help line `0x140303B30`: row 0 → the weapon's help (`0x1402E6A10`), else the accessory at
  `0x142E9AA6C − 1` (KH1 ignores the scroll there).

### The weapon list `0x140303090` (high)

- Entries `0x142E9AAB0` (ints, up to 256), counts `0x142E9AEB0`, count `0x142E9B2B0`; the
  per-slot weapons `0x142E9AA80` (the model loaded) and `0x142E9AA90` (chosen), both set from the
  records as the page opens (`0x140304C30`).
- While the cursor rests on another weapon (party slots 0..2), `0x142E9AAA8 = 1` and the preview
  window's update `0x140302BA0` puts it in Sora's hand with `0x1402845A0` (which writes `+0x36`).
- Confirm: `0x140304BC0`, then `R + 0x36 = item` (through `0x142E9AA50 + 0x32`),
  `0x142E9AA90[slot] = item`, stats refreshed (`0x1402E5A80`), state 3, `0x142E9AA60 = item`.
- Cancel / close (`0x140302AF0`): if the loaded model isn't the chosen weapon, `0x1402845A0` puts
  the chosen one back, then `R + 0x36 = chosen`.

### Proposal: a "Valor" row under Sora's weapon row

The plan, in the style of the shortcut work (`docs/kh1-shortcuts.md`); what was built, and where
it differs, is in the next section.

1. **Where the id lives**: `save + 0x488` (section 5). `equip::valor_keyblade` reads it and, if it
   is no longer a free Keyblade (or holds anything else, as on old saves), picks the free one with
   the most Strength. None → Valor can't be entered (`valor_candidates` empty). A Keyblade picked
   as Sora's main weapon leaves the candidates, so the next read refills the row.
2. **Rows**: for Sora, `accessories + 2`. Easiest: swap the update pointer in the slots window's
   definition (`0x1405491A0`, a data qword) for a wrapper that, before KH1's update, fixes a Sora
   window still at `acc + 1` rows (`+0xF8`, `+0xFA` ≤ 8, `+0xE6`), so the three count sites need
   no patch.
3. **Row items**: the slot window reads row `n`'s item only through `r12 + n + 0x1C`
   (`0x140303E96`, `0x140304001`). Patch the 7 bytes at `0x140303C15` into a jump to a stub that
   sets `r12` to a 0x30-byte shadow of ours for Sora (`shadow[0x1D]` = the Valor Keyblade,
   `shadow[0x1E + i]` = accessory `i`; refreshed by the wrapper each frame), else KH1's
   `save + member × 0x74`. Row 0 (weapon), row 1 (Valor) and rows 2.. (accessories) then draw.
4. **Label**: repoint the call at `0x140304F74` (`call 0x1402E6570`) to ours: while the slots
   window's wrapper is running and the item is the Valor Keyblade, return our text (menu
   `encode`: "Valor " + KH1's name codes from `equip::item_name_ptr`; the row is `0xA6` px wide,
   so "Valor Form: Ultima Weapon" may not fit), else KH1's name. The page header uses the same
   function, so the header shows the label too.
5. **Confirm**: in the wrapper, before KH1's update: Sora, confirm (`0x1402EDF00`'s test), row 1
   → set `+0xFE` so KH1 sees row 0 for this call (it opens its weapon list, compare box and
   preview), put the cursor back after it, set `0x142E9AA5C` to the Valor Keyblade and a "Valor
   pick" flag. Row ≥ 2 → after KH1's update, `0x142E9AA4C = row − 2` and `0x142E9AA5C` = that
   accessory (KH1 took row − 1). The page's rebuild (`0x140304360` state 2 recomputes
   `0x142E9AA4C` at `0x140304592`) needs the same fix (wrap `0x140549200`'s pointer).
6. **The list** (wrap `0x140549140`'s pointer, Valor pick only): after KH1's fill, keep only
   `valor_candidates` (and the current Valor one) in `0x142E9AAB0 / 0x142E9AEB0 / 0x142E9B2B0`.
   On confirm, store the cursor's id at `save + 0x488` and undo KH1's equip after its update:
   `save + 0x36` and `0x142E9AA90[slot]` back to Sora's weapon, `0x1402E5A80` again. The preview
   showing the hovered Keyblade in his hand is harmless: closing (`0x140302AF0`) puts the chosen
   weapon (`0x142E9AA90`) back. The compare box then compares with `0x142E9AA5C` (Valor's).
7. **Help**: wrap `0x140549080`'s pointer: row 1 → the Valor Keyblade's help
   (`0x1402E6A10 (id)`), rows ≥ 2 → KH1's with `0x142E9AA6C` one less around the call.
8. The Status page (`0x14030FFD0`) shows Sora's weapon from `R + 0x36` (`0x1403104A0`); a Valor
   line there is optional.

Things to check in the game first: that the wrapper's row fix lands before the first draw, and
the scrolled case (Sora with 7 or 8 accessories gives 9 or 10 rows, KH1 scrolls past 8).

### Implemented (`crates/kh1-hook/src/win/equipmenu.rs`)

The plan above, with these changes once the code was read closer:

- **Confirm** doesn't fake the cursor: `0x1402EDF00` (the confirm test) keeps state in the window
  (`+0xE8`), so it can't be asked twice. Instead the row test at `0x140303CA4` (`test ebx, ebx;
  jne <accessory>`, `ebx` = the row) jumps to a stub: row 0 → KH1's weapon way; row 1 with a
  Valor Keyblade → the weapon way with a "Valor pick" byte set; row 1 with none → sound 4 (KH1's
  refusal), then the drawing part (`0x140303F87`); other rows → the accessory way. Rows ≥ 2: KH1
  sets `0x142E9AA4C = row − 1`; right after its update ours makes it `row − 2` (`0x142E9AA5C` is
  already right, as KH1 reads it through the shadow `r12`).
- **Rows** are fixed at the start of the slots update and also right after the members' update
  (it sets its child's rows as its cursor moves): only a window at exactly `acc + 1` rows is
  changed, to `acc + 2` (visible ≤ 8, height KH1's formula).
- **The list fill** `0x140304E50` is hooked (its prologue) rather than filtering after the list
  window exists: the window takes its row count and cursor (`0x142E9AA70 / 74`, set by us to the
  current Valor Keyblade) when it is made.
- **The live preview during a Valor pick** (changed after the user's report: "in the menu when
  selecting the second keyblade, it doesnt display the correct keyblade"). The first version
  cleared KH1's preview request, so the preview panel kept Sora's own Keyblade. KH1's preview
  is kept now. `0x142E9AAA8 = 1` → `0x140302BA0` loads `0x142E9AA80[slot]` into Sora's hand
  (`0x1402845A0`, which writes his weapon byte), then puts its model in the panel
  (`0x1402D86B0`). KH1 undoes it itself: the list's close handler (`0x1403033D0`, the list
  definition's second qword) opens a window (`0x140548F60` → `0x140302AF0`). Once no load is
  in flight (`0x142E9AAA4 == 0`), that window loads `0x142E9AA90[slot]` again if it differs
  from `0x142E9AA80[slot]`, and writes `+0x36` = that.
  - On confirm (state 3), `0x142E9AA90[slot]` and `save + 0x36` are put back to Sora's own
    weapon; `0x142E9AA80[slot]` is left as the preview's, so the restore runs. Then
    `0x1402E5A80` refreshes the stats, and the pick (`0x142E9AA60`) is stored at
    `save + 0x488` if it is a free Keyblade.
  - Sora's own weapon is noted when the list opens (the fill). Until his byte holds it again
    with the list closed, two things use it instead of the preview's byte: the Valor slot
    isn't re-decided (`valor::keyblade` would see the previewed Keyblade as equipped), and
    the fill counts free Keyblades with his own weapon.
- **Label**: "V: " + KH1's name codes for the item (short, to fit the `0xA6` px row), only while
  our slots / page wrappers run, only for the Valor Keyblade's id on Sora's page.
- **Help**: for Sora in state 1 KH1's help runs with the scroll at 0 and the cursor at the row
  KH1 expects; on row 1 Sora's first accessory byte (`save + 0x1D`) holds the Valor Keyblade for
  the call (put back after), so KH1 shows its help.
- The Valor Keyblade shown is `valor::keyblade()` each frame (`equip::valor_keyblade`, written
  to `save + 0x488` when it changed).
- Everything is checked first (the five window definitions hold KH1's functions, the bytes at
  `0x140303C15`, `0x140303CA4`, `0x140304E50`, the call at `0x140304F74`); any mismatch logs and
  leaves KH1 as it is. The two code jumps go in last.

Log lines (`kh1-kh2.log`, all start with "Equipment:"): the install, "Valor's Keyblade is ..
(candidates ..)" on change, "Sora's slots N rows", "Valor row picked", "weapon list for Valor:
[..] -> [..]", "Valor's Keyblade stored: .." or "not stored", "Sora's row N is accessory slot K".

To test in the game:

1. Equipment → Sora: a row "V: <Keyblade>" under his weapon; his accessories below it; the
   other members unchanged. Move the cursor over each row: the help line matches the row.
2. Confirm on the V row: KH1's weapon list with only Keyblades Sora owns and isn't holding,
   the cursor on the current one; resting on one doesn't change the Keyblade in his hand.
   Confirm one: the row shows it, Sora's weapon (row 0 and in his hand) is unchanged, Strength
   unchanged. Cancel instead: nothing changes.
3. Confirm on an accessory row: the right accessory slot is replaced (first, last).
4. Equip the V row's Keyblade as Sora's weapon: the V row moves to another Keyblade.
5. With only the Kingdom Key (or in Dive to the Heart): the V row is empty and confirm on it
   buzzes.
6. Sora with 7 or 8 accessory slots (9 / 10 rows): scrolling, the last accessory, its help.
7. Steam build: the same, and the install line without "some patches failed".

## 5. A spare save byte for the Valor Keyblade

**`save + 0x488`** (and the three bytes after it):

- The ten party records end exactly there (`10 × 0x74 = 0x488`); `+0x48C / +0x48D` are bytes of
  their own (written by `0x1402835B0 / 0x1402835A0`), then the party slots `+0x48E`.
- A scan of every instruction in `.text` with a displacement `0x480..0x48B`: none follows a load
  of the save pointer; the game-code hits are other structures (actor handles at `+0x488`, menu
  windows, floats). New game (`0x140283150`) writes the dword at `+0x48E` (`0x1402832E8`) but nothing
  at `0x488..0x48B`. The
  record loops stop at 10 members (`0x1402E5A80`, `0x1402E6330`). (medium-high: a static scan
  can't see an access through a pointer computed elsewhere, as with `+0x847`.)
- It travels with the save like `+0x847` (the whole block is the save data).
- Old saves may hold anything there, so the id is checked on every read (`valor_keyblade`). If a
  marker is wanted, `+0x489` can hold one.
- `+0x489` is Master Form's Keyblade since t1025 (the owner's answer 1 after Master's research round
  2, `docs/kh1-drive-forms-native.md` §24): checked on every read the same way (`equip::master_holds`:
  a Keyblade Sora owns with a copy free of the members and of Valor's slot), and a value that isn't
  one is cleared (`valor::master_slot`).
- `+0x48A..+0x48B` (a u16) is the abilities' layout mark (`growth::LAYOUT_OFF`,
  `docs/abilities.md`). A second scan found no `0x489..0x48B` displacement, immediate or absolute
  address (`0x142DEA1E9..0x142DEA1EB`) anywhere in `.text`.

Not usable: `+0x840..0x843` (a u32 KH1 sets to 1), `+0x848..` (a u16 array of counters indexed
from `+0x848`), `+0x37..+0x3B` of Sora's record (no direct use found, but the record has uses
through four different bases, so a scan is weaker there), `save + 0x499` (the count of item 0:
KH1's loops start at 1, but the add / take functions index by any id).

## Steam addresses

Functions: `0x14028D7E0` → `0x14028F970`, `0x14028D820` → `0x14028F9B0`, `0x140284880` →
`0x140286A00`, `0x140284960` → `0x140286AE0`, `0x1402845A0` → `0x140286720`, `0x140284BA0` →
`0x140286D20`, `0x1402A2FA0` → `0x1402A5130`, `0x1402A2CC0` → `0x1402A4E50`, `0x1402E5A80` →
`0x1402E7EC0`, `0x1402E6330` → `0x1402E8770`, `0x1402E6BC0` → `0x1402E9000`, `0x1402E6C60` →
`0x1402E90A0`, `0x1402E6570` → `0x1402E89B0`, `0x1402E6A10` → `0x1402E8E50`, `0x140304C30` →
`0x140307070`, `0x140304E50` → `0x140307290`, `0x140304F30` → `0x140307370`, `0x140302AF0` →
`0x140304F30`, `0x140304BC0` → `0x140307000`, `0x140282CF0` → `0x140284E60`, `0x14038B0B0` →
`0x14038ADC0`.

Patch sites: `0x140303C15` → `0x140306055`, `0x140303CA4` → `0x1403060E4`, `0x140303E8A` →
`0x1403062CA`, `0x140303E96` → `0x1403062D6`, `0x140304001` → `0x140306441`, `0x14030402E` →
`0x14030646E`, `0x1403041A5` → `0x1403065E5`, `0x1403043EF` → `0x14030682F`, `0x140304850` →
`0x140306C90`, `0x140304592` → `0x1403069D2`, `0x140304F74` → `0x1403073B4` (checked: the same
instructions and bytes in the Steam exe).

Data: item table pointer `0x142D23738` → `0x142D22D38`, `btltbl.bin` `0x142D23740` →
`0x142D22D40`, word handles `0x142D23998` → `0x142D22F98`, text pages `0x142EE4730` →
`0x142EE3980`, weapon table offsets `0x140528D60` → `0x140528060`, weapon kinds `0x140545FC0` →
`0x1405452D0`, locked weapons `0x140546020` → `0x140545330`, menu state `0x142E9AA30..0x142E9AAA8`
→ `0x142E9A050..0x142E9A0C8` (same order, −`0x9E0`), lists `0x142E9AAB0` → `0x142E9A0D0`,
`0x142E9AEB0` → `0x142E9A4D0`, `0x142E9B2B0` → `0x142E9A8D0`, menu stats `0x142E94BC0` →
`0x142E941E0`.
