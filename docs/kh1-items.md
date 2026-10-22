# KH1's items, the Items page and the shops, and KH2's Drive Recovery items among them

The user: "have drive recovery items exist and be sold at the item shop in traverse town". The
items are KH1's own: two records in KH1's item table, counted in KH1's inventory, used from
KH1's Items page, sold by KH1's shop. Code: `crates/kh1-hook/src/driveitems.rs` (the numbers)
and `crates/kh1-hook/src/win/items.rs` (the hooks).

Read statically (objdump of the Epic executable, image base `0x140000000`; every address the mod
uses is in `steam_map.in`, and the three hooked prologues are the same bytes in the Steam
build). KH1's data was read from the user's install (`btltbl.bin` from the packages, the rooms'
`.ard` scripts on the PC, nothing kept). No decompiled code here.

Confidence: **high** = read in the code, every step seen; **medium** = one step inferred.

## 1. KH2's Drive Recovery items (high)

From the user's `03system.bin` (`item`, 24-byte rows, OpenKH's layout) and `msg/us/sys.bar`:

| KH2 id | Name | Type | Byte 4 | Buy | Sell | Command | Description |
|---|---|---|---|---|---|---|---|
| 274 | Drive Recovery | 1 | 3 | 0 | 20 | 0 | "Restores up to three full Drive Gauges. Usable only from the Main Menu." |
| 275 | High Drive Recovery | 1 | 9 | 0 | 30 | 0 | "Restores all Drive Gauges. Usable only from the Main Menu." |

(Type 0 is the consumables, Potion to Megalixir; type 1 has no command, so they never go in the
battle menu or the item slots.)

KH2's executable (the copy disassembled for these notes):

- **The effect** (`0x1403C4B80`, the item-use switch; for 274 and 275 one case): the gauge's add
  `0x1403C0AD0 (gauge, byte 4 × 100)`, the same add as a hit's or an orb's. So +300 and +900
  points: a part-filled bar carries over, and the gauge holds at its maximum (100 points in the
  last bar). The add touches only the bars and points (`+0x1B0`, `+0x1B1`), so **in a form it
  fills the bars waiting under the form's gauge, not the form's gauge** (`+0x1B4`).
- **When it can be used** (`0x1403C3DA0`, the usable switch, the same two cases): the bars
  (`+0x1B1`) below the maximum (`+0x1B2`). Nothing about the form: usable in a form too while
  the bars under it aren't full. With no Drive gauge yet the maximum is 0, so never.
- **Prices.** Neither has a buy price, and none of KH2's 21 shops (03system `shop`, 404 products)
  lists them: KH2 doesn't sell them. KH2's sell price is a quarter of its buy price for every item
  it sells (Potion 40 / 10, Hi-Potion 100 / 25, Ether 120 / 30), so the sell prices 20 and 30
  give **80 and 120 munny** (derived: medium). 00battle has nothing for items.

## 2. KH1's item table (high)

`btltbl.bin` is loaded into the exe's own buffer `0x142D23740` (`0x14028D840`), and pointers into
it set: the item table `*0x142D23738` = `+0x1A58`, 255 records of `0x14` bytes, item `id`'s at
`table + (id − 1) × 0x14` (`0x14028D7E0`). `docs/kh1-equipment.md` has the record; for items:

| Offset | What |
|---|---|
| `+0x00` | u16 the name's word; the name is the handle `0x142D23998[word]` (`0x14028D820`); item ids use words `id + 30` |
| `+0x02` | kind: 0 a consumable (goes in item slots), 1 an accessory, 2..11 a member's weapon, 12 used from the menu only (Tent, Camping Set, Cottage, Power / Defense / AP Up), 13 synthesis and key items |
| `+0x03` | the item's field model (`0x1402BDDEB`, table `0x14052ACF0`): Potion / Ether 0, Hi-Potion 1, Elixir 2 |
| `+0x04` | flags; bit 0 used on the whole party |
| `+0x06` | the amount (Potion 30, Ether 3, Elixir 255) |
| `+0x08` | the buy price (`0x1402E64B0`; the shop's cost × count) |
| `+0x0A` | the sell price (`0x1402E65E0`): a fifth of the buy price (Potion 60 / 12, Tent 250 / 50) |
| `+0x0C` | the help text's handle (`0x1402E6540`) |
| `+0x10` | the sort key (`0x1402E6600`, the lists' sort `0x14030C130`): Tent 101, Cottage 103 |

- **The icon** is by kind (`0x1402D3AB0`: icon `0x14052BD40[kind]`, colour `0x14052BD78[kind]`;
  kind 0's is `0x1B`, kind 12's `0x1D`), read through `0x1402E6680` there only.
- **Unused records**: id 5 (kind 0, "E05"), 0x39, 0x48..0x50, 0x76, 0x86, 0x88, 0x8A, 0x8B, 0x8D,
  0x91..0x94 (kind 12, "F04".."F07"), 0xE8: help `0xCC40` (empty) and sort key 10000. Id 5 is in
  the kind-2 shop's lists (op 0x274, below), so not that one; 0x91..0x94 are in no shop list and no
  prize-table row (`btltbl` + 0xC6A8).

## 3. The inventory and the Items page (high)

- One count per id at `save + 0x499 + id` (1..255, at most 99, `0x140282CF0`). A kind-0 item
  bought or found goes in a member's item slots first; the rest only count.
- **The Items page** (pause menu): picking an item opens its command window (`0x14030A600`):
  Use (greyed unless `0x1402E66E0`: usable on someone in the party), Equip (kind 0 only, the item
  slots), and the rest. Use on a single-target item asks for the member; a party item doesn't.
- **Usable on a member**: `0x1402CB1F0 (item, member)`: kind 0 or 12 only, then a switch on
  `id − 1` (byte table `0x1402CB328`, ids 1..0x9A): HP items need HP below the maximum, MP items
  MP, Elixirs either, the stat ups the stat below 100. Every other id (5, 0x91.., ...) is never
  usable. Bit 0 of the answer is "usable".
- **The use**: `0x14030CD20 (page, slot member or −1, slot or item, target slot)`: the effect on
  each target (`0x1402CB3E0` → `0x1402CB5D0 (item, 0x10000 | member)`, which writes the save's
  record and returns a bit per effect: 2 HP, 4 MP, 8 both, 0x10 / 0x20 / 0x40 the stat ups). With
  no bit: the error sound (4) and a window with message `0xA0`. With one: the item taken (its
  slot cleared, or the count less one), the menu's stats refreshed (`0x1402E5A80`), sound 0xE
  (7 for a stat up), and the result window (descriptor `0x140549A00`) with message `0x99 + bit`
  and the member (`+0x48`, `+0x4C`), sized by its line count (`0x1402CE890`). The window draws
  its line through `0x1402EAEA0` → `0x1402CE7C0 (message)`.
- No field animation: a menu use is the sound and the window. KH1's battle-menu Items list is the
  member's item slots, so kind-12 items never show there (nor in the party's AI).

## 4. The shops (high)

- Room scripts open a shop through script ops (the op table `0x1404FB550`): op 0xDE
  (`0x1401C5E10`) is **the item shop** with list `n` from `0x1404FF3C0 + n × 0xD4` (a u32 count,
  then up to 52 u32 ids), through `0x1402FEC60 (0, list, ...)`; op 0x274 (`0x1401C5B40`) another
  kind (2) with lists at `0x1404FFA60`; ops 0x10C and 0x275 open kinds 1 and 3, without a list.
- The item shop's lists: 0..3 Potion, Hi-Potion, Ether, Tent, then more as the story goes (Camping
  Set, Elixir... and Donald's staves and Goofy's shields); 4..6 accessories; 7 Potion, Hi-Potion,
  Ether, Tent, Camping Set.
- Who opens which (all `.ard` scripts searched): **room `tw10`** (Traverse Town, world 3 room
  10, the Item Shop) opens lists 0..3 by its story byte; `tw11` (the Accessory Shop) 4..6;
  `al19` (Agrabah) 7; `tw01` op 0x274. Worlds are numbered by KH1's prefix table
  (`0x1404DD980`: dh, di, dc, tw, ...).
- Buying (`0x1402FF220`): price `+0x08` × count against the munny; a kind-0 item asks which
  member's slot. Selling (`0x1402FF380`): every id with a free copy and a sell price `+0x0A`.

## 5. What the mod adds

- **Ids `0x91` Drive Recovery and `0x92` High Drive Recovery**: two of KH1's unused kind-12
  records (after Cottage `0x90`). After each table load (`menu`'s hook on `0x14028D840`) and before
  they are drawn, used or sold, their records are written in place: kind 12, model 0 / 2
  (Ether's / Elixir's), no party flag, amount 3 / 9, **buy 80 / 120, sell 16 / 24** (KH1's fifth),
  our help text (KH2's descriptions), sort 104 / 105; and their name words (175, 176) get our
  names in `0x142D23998`.
- **Icon**: the call of `0x1402E6680` in the icon draw (`0x1402D3ADD`) answers 0 for them: the
  consumables' icon.
- **Usable** (`0x1402CB1F0`, hooked): on Sora (member 0), with the Drive option on, while the
  gauge is shown (a Drive Form open) and its bars are below the maximum (KH2's test). In a form
  the bars are the ones waiting under the form's gauge.
- **Use** (`0x14030CD20`, hooked): on Sora and usable, KH1's success path with the gauge for HP:
  `gauge::recover` adds 300 / 900 points, one is taken, the stats refresh, sound 0xE, the result
  window with "Sora's Drive Gauge was restored." (message `0x7F01`, answered by the redirected
  call at `0x1402EAECB`). Otherwise KH1's own path: its use switch does nothing for these ids, so
  the error sound and its "can't use" window. The log has each use with the gauge before and
  after.
- **Shop** (`0x1402FEC60`, hooked): when the item shop opens in world 3 room 10 and a Drive Form
  is open, the list it gets is KH1's with ours after it (a list of ours, at most 52). Every shop
  opening is logged with its world, room and list. Other shops are untouched.

## 6. Saves

- The counts are KH1's own bytes (`save + 0x499 + 0x91`, `+ 0x92`); nothing else is stored.
- **Without the mod** (read from KH1's code, medium for what the name shows): the records are
  KH1's placeholders, so the Items page lists them under the placeholder names of words 175 / 176,
  with KH1's kind-12 icon; "Use" is greyed (the usable switch has no case for them); they can't be
  equipped (kind 12) or sold (sell price 0, so the sell list leaves them out). Loading isn't
  affected: the bytes are plain counts.

## Open

- That the result window's line reads well (its code set and length): the log has KH1's own
  Potion line once ("KH1's own line for a Potion reads ..."), and the records' names and help.
- That KH1 allows the pause menu in battle everywhere (KH2 does; KH1's own rules decide).
