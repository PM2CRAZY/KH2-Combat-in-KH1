# KH1 shortcuts (L1 + a face button)

Research notes (Ghidra, Epic KH1 Final Mix PC, image base `0x140000000`) for KH2's fourth
shortcut (`crates/kh1-hook/src/win/shortcuts.rs`). Read statically; the drawing changes are
checked in play.

## Storage

Sora's three shortcuts are bytes at `save + 0x844 .. 0x846` (`save = *0x1428695A0`): a spell
0..6 (Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero) or `0xFF` for none.

- `0x140282F20 (slot) -> i8`: get. `0x140283640 (slot, spell)`: set (Customize).
- `0x1402830F0 (member, spell)`: a spell learned; for Sora it fills the first empty slot of three.
- New game (`0x140283150`) writes `0xFFFF` at `+0x844` and `0xFF` at `+0x846`; `+0x847` is
  never read or written (padding before the u16 at `+0x848`), so the fourth is kept there, as
  spell + 1 (0 = none, which is what old saves have).

## Battle

- The battle menus are filled each frame by `0x140292330` (called from `0x140292090`, from the
  command step `0x140291660`). The magic menu is `0x140529638` (entries `0x14052964C`, count
  `0x140529648`, disabled bits `0x14052963C`); the shortcut menu is the command list at
  `0x140529718`: type 5, `+4` / `+8` disabled bits, `+0xC` cursor (-1), `+0x10` count (3 in
  the exe's data), `+0x14` command ids (`level + (spell + 2) * 3`, `0x42` for none).
  `0x142D5445C` is set when any shortcut is set.
- Logical buttons (`0x140288F30`): held `0x14298D4A4`, pressed `0x14298D4A8`. Triangle gives
  `0x80000`, Square `0x100000`, confirm `0x210001`, cancel `6` (confirm / cancel are Cross /
  Circle, or the other way round with the button swap, `0x1402F4690`). L1 held is `0x40000`.
- The command step switches its root (`0x142D543F0`) to the shortcut menu while L1 is held (and
  a shortcut is set). There, with no command picked (`0x142D543E8` set to command `0x43`'s
  record), the bits `0x80000 / 0x100000 / 0x200000` run entries 0 / 1 / 2 through
  `0x1402912F0 (menu, entry)`. Its jump check (pressed `& 4`) comes earlier and ends the step.

## Drawing

`0x140279700`, case 5 draws the shortcut menu at x `[0x14232E6CC] + 0x10`, y `0x11A` (one row,
22 px, lower than the four-row command menu at `0x104`): two panels (`0x140278880` with the
sprite defs `0x1427F1ED8`, `0x1427F1EA8`), three button icons (`0x140277E50`, text icons
`0x46` Triangle, `0x45` Square, `0x41` confirm; `0x42` is cancel; both follow the swap,
`0x1402D26F0`), then three rows (`lea ebp, [rdi - 0x11]`) of command names 22 px apart.
`0x140278880` builds one quad at `0x142867360` (`0x14027B300`; UVs at `+0x20..`, colours
`+0x60..`, positions `+0xA0..`: top-left, bottom-left, top-right, bottom-right, 1/16 px) and
submits it to the draw list `0x142853218` (`0x140274340`).

## Customize

The shortcut list is window `0x140548E40` (rows / visible rows at `+0x26` / `+0x28`), drawn by
`0x140300510` (three rows, labels sysmsg `0xCE / 0xCF / 0xD0` from `0x140548D40`); a row picks a
spell in `0x140300300`, which calls the set. The overview `0x1403010E0` shows the same three
(labels `0x140548F50`).

## Who reads the slots (Checked, exe)

Only these touch `save + 0x844 .. 0x846`: the get `0x140282F20` (called by Customize's shortcut
list `0x140300510` and overview `0x1403010E0` only), the set `0x140283640` (from the spell list
`0x140300300`), the learn `0x1402830F0`, new game `0x140283150`, `0x140282FB0` (writes 0, 1, 2),
and the battle fill `0x140292330`, which reads the bytes directly: for a slot it calls
`0x14028D0D0 (spell)` (the spell table, 7 entries of 0x38 bytes from `0x142D23580`) and indexes a
local array of 8 by the spell. So a slot byte of 7 would make KH1 read past its spell table and
an uninitialised local, and build a stray command id. `+0x847` is not read or written by KH1.

## Customize's spell list (Checked, exe)

- The list is ints at `0x142E9A9F0`, count `0x142E9A9E4`, cursor `0x142E9AA18`, the slot being set
  `0x142E9AA14`; right after the list, `0x142E9AA10` is the overview's party member. The fills
  `0x140301D60` (called by `0x1402D8AA0` as the pause menu opens) and `0x140301E50` (same body)
  put the learned spells (`0x1402830C0 (0, spell)`) and then 7 for "none": at most 8, all the
  room there is. "None" is told by place (the last row), not by value, everywhere.
- The list window (definition `0x140548E10`, update `0x140300300`) takes its row count from the
  count; its rows are drawn `[0x140546050] + [0x140546058]` apart (every menu's row height and
  gap). Confirm calls the set with the entry (or -1 for the last) and keeps its name
  (`0x14028D100`). Its help line `0x140300260` shows sysmsg `entry × 3 + 0xD4 + 0x14028D0D0
  (cursor)` (KH1 passes the cursor, not the spell, to the level), `0xEA` for "none".
- Names: `0x14028D100 (spell)` returns 0 for a spell past 6 (the callers then show sysmsg
  `0xD4`, "none"), else KH1's word for the command `level + (spell + 2) × 3`. Called at
  `0x1403003DD`, `0x14030042B` (the list), `0x1403006F0` (the shortcut list), `0x140301177` (the
  overview).

## Reflect in the shortcuts (Ours, `shortcuts.rs`)

- Saved: a slot 0..2 holding Reflect keeps KH1's none (`0xFF`) with bit `4 + slot` of `+0x847`
  set; the fourth's nibble is 8. KH1 (and the mod without Reflect) sees none there; a bit whose
  byte isn't `0xFF` means nothing. The get / set hooks are in with `shortcuts = 3` too.
- KH1's learn sees Reflect's slots as taken (a placeholder `0xFE` for the call), so a new spell
  doesn't land on them.
- Battle: after KH1's fill, a Reflect slot's entry becomes command 0x7D (greyed as the Magic
  menu's Reflect: magic allowed, not sealed, 1 MP, KH2 has Sora) and `0x142D5445C` is set.
  L1 + its button runs it through `0x1402912F0` like any entry: KH1's command record gets type 2,
  spell 7, and `combat::command` hands it to `reflect::chosen` (KH1's spell code never sees it).
  The fourth shortcut (L1 + cancel) the same.
- Customize: the four `lea` instructions that address the list (`0x14030029A`, `0x140300322`,
  `0x140301D86`, `0x140301E5C`, all checked first) point at a 16-int buffer of ours near the
  game; after each fill Reflect (7) goes before "none" once Sora has it. With 9 rows the list's
  update runs with rows 25 apart (gap 0), as Config's nine. The help line's level call
  (`0x14030028E`) returns, on Reflect's row, what makes the id ours (`0x7F000006`, answered by
  the menu text lookup). The four name calls answer 7 with Reflect's level name.
- Not checked in the game: the 9-row list's look (only the rows are pitched; the window has no
  height of its own in its definition).

## Limit Form's shortcuts (Ours, `limits.rs`; KH2's layout read from its executable)

- **KH2 FM** (PC executable, read): in Limit Form KH2's shortcut fill (`0x1403F2780`; the same
  code is in the command menu build at `0x1403F2210`) reads each of the four saved shortcuts
  (`0x1409ACA28`, four u16 commands) and, unless the slot holds an item, shows entry k of the
  table at `0x1405C97E8` in slot k: Sonic Rave, Last Arcanum, Strike Raid, Infinity (`cmd` 0x2BA,
  0x2BD, 0x2C0, 0x2AB). The saved shortcuts are left as they are; the form only shows these.
- **Here:** while Sora is in Limit Form (with KH2's Limit menu off), after KH1's fill
  (`limits::fill_kh1`, from `drive::fill`) slot k of the shortcut menu holds the KH1 limit
  standing in for KH2's k-th: L1 + Triangle Sonic Blade, L1 + Square Ars Arcanum, L1 + confirm
  Strike Raid, L1 + cancel (the mod's fourth) Ragnarok. The table is `limits::SHORTCUT_LIMITS`
  (indices into `limits::KH1_LIMITS`; Trinity Limit is 4). Each entry is the Limit menu's record
  for that limit (magic-type, its own argument, KH1's word for the limit, so KH1's name is drawn),
  greyed as that menu's row. The shortcut flag `0x142D5445C` is set, so L1 opens the menu with
  no spell in a shortcut. `shortcuts::fill` leaves the menu alone that frame (no Reflect slot, no
  fourth spell over it). KH1's three slots never hold items, so every slot is a limit.
- L1 + a button runs the entry through `0x1402912F0` as any shortcut; `combat::command` turns it
  into KH1's own limit command (type the limit's command id), which KH1's command step runs.
- The save's shortcuts are not touched. KH1's fill rebuilds the menu every frame from them, so
  Sora's own shortcuts come back the frame the form ends.
- With `shortcuts = 3` the menu has three slots and Ragnarok none.
- Not checked: which face button KH2 shows for its slots 2 and 3, against ours (confirm,
  cancel).
