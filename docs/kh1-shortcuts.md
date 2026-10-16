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
