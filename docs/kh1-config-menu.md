# KH1 Config menu: where the options live and how to add one

Research notes from the user's KH1 Final Mix executable (Epic 1.5+2.5, x64, image base
`0x140000000`, all addresses absolute), Ghidra headless. Plain-language findings only; no
decompiler output. Goal: a new Config row "Camera Style: KH1 / KH2" implemented by
`kh1-hook`.

Confidence tags: **[high]** read directly from code and data, consistent across several
places; **[med]** strongly suggested by how the code uses it; **[low]** inference, check in
game before relying on it.

## 1. Where the option values live

### 1.1 The config block [high]

- The save data is one contiguous region starting at `0x142DE9D60` (the block `menu.rs`
  already uses). Saving and loading read or write **0x16C00 bytes** from there (the save
  file per slot, the PS2 `BISLPS-25198xx/system.bin` naming kept). So everything up to
  `0x142E00960` is per save slot.
- `0x1402CCC70(n)` returns the save sub-blocks: 0 `0x142DE9D60` (main), 1 `0x142DEA960`,
  2 `0x142DEB960`, 3 `0x142DEBD60`, 4 `0x142DEC160`, **5 `0x142E00160` (config, 0x400 bytes)**,
  6 `0x142E00560`, 7 `0x142E00588`.
- At start-up `0x1402CD850` stores sub-block 5 in the pointer **`0x142E1FE70`**. All Config
  code goes through that pointer. So the Config options **are saved in the save file**, per
  slot. They are not in a separate system file. The launcher's `config28.dat` /
  `config1525.dat` (`0x14012BC60`) are the collection's PC settings, not this menu.
- New game: `0x1402CBC10` zeroes the whole save region, config block included (it writes up
  to `0x142E00558` and beyond). Then `0x1402CD8D0` sets the defaults (`0x1402F8C00`): all
  options 0 except the mode (`+0x2C`) = 1, and `+0x1C` = 0.
- After a load (`0x1402DE840` and the load menu `0x1402F5080`) the game re-derives the two
  camera-inversion floats from the loaded values (see `+0x14`, `+0x18`).
- The block's laid-out part is **0x78 bytes**. The save's byte-order converters
  (`0x1402CD470` / `0x1402CD630`, format string at `0x1403F0B38`:
  `8i 12c 1i 2i 1i 1i 10i 1i 1i 1i 1i`) cover `0x142E00160..0x142E001D8` only.
  **`+0x78..+0x3FF` (904 bytes) is never read or written** except by the new-game zeroing.
  No absolute reference and no pointer-based access past `+0x74` was found [med: absence
  of evidence; every reader of `0x142E1FE70` was checked].

### 1.2 Layout of the config block (offsets from `0x142E00160`)

| Off | Size | Meaning | Values | Conf |
|---|---|---|---|---|
| +0x00 | i32 | Config row 0 (has a pad-diagram sub-window) | 0 / 1; read by `0x1402F8B70` (player action code `0x1402912F0`, `0x1402AF020`, which act when the value is not 1; targeting / command related) | high (storage), low (name) |
| +0x04 | i32 | Config row 1 (pad-diagram sub-window) | 0 / 1; read by `0x1402F8B60` (`0x1402AE480`: with 1, a lock-on-state check uses the other edge of a button bit, so probably hold vs toggle) | high / low |
| +0x08 | i32 | Config row 2 (pad-diagram sub-window): **Field camera** | 0 = Auto (the auto-follow camera code `0x1401DF7C0` / `0x1401E4510` runs only when 0), 1 = Manual; getters `0x1402F8B90`, `0x1402F8C90`, setter `0x1402F8E70` | high / med |
| +0x0C | i32 | not in the menu, reset to 0 by defaults | ? | high (exists) |
| +0x10 | i32 | Config row 3: **Vibration** | 0 = on (the rumble code `0x140288F30` drives the motors only when 0), 1 = off; getter `0x1402F8BF0`, setter `0x1402F8E90` | high |
| +0x14 | i32 | Config row 4: **Camera up/down** | 0 / 1; also sets float `0x140507AA8` = 2·v − 1 (−1 normal, +1 reversed), which multiplies the pitch input (`0x1401E0230`, `0x1401E51C0`); getter `0x1402F8A50`, setter `0x1402F8B00` | high (storage), med (axis) |
| +0x18 | i32 | Config row 5: **Camera left/right** | 0 / 1; also sets float `0x140507AAC` = 1 − 2·v, which multiplies yaw input (`0x14028A650`, `0x1401E7140`, `0x1401E51C0`); getter `0x1402F8A60`, setter `0x1402F8B30` | high / med |
| +0x1C | i32 | not Config (reset at new game; used by `0x1402E59E0`, `0x1402E6430`, `0x1402E6C30`) | | high |
| +0x20 | 10 × u8 | per-character byte (party / AI screens `0x140300790`, `0x140301C80`; default 4) | | high |
| +0x2A | 2 × u8 | padding in the `12c` group, unused | | med |
| +0x2C | i32 | Config row 6: **Mode** (Beginner / Standard / Proud), read-only in the menu (pressing it plays the buzzer) | 0..2, default 1; getter `0x1402F8BA0` (14 gameplay callers), set by the new-game screen (`0x1403095B0`, setter `0x1402F8E80`); end-of-game awards in `0x1401C7700` test it | high (storage), med (name) |
| +0x30..+0x74 | | Journal flags and bitfields (`0x1402D90A0` clears them; `0x1402D9010` / `0x1402D92A0` bit arrays at `+0x3C`; `+0x6C` bit array; `+0x70` bit 0 = "new journal entry" marker on the pause menu; `+0x74` read by the journal) | | high |
| +0x78..+0x3FF | | **unused, saved, zeroed at new game** | | med |

The menu reads and writes the rows only through `0x1402F8EC0(row)` and `0x1402F8F90(row,
value)`. These map rows 0, 1, 2, 3, 4, 5 to `+0x00`, `+0x04`, `+0x08`, `+0x10`, `+0x14`,
`+0x18`. **Any other row number falls through to `+0x2C` (the mode).** This is the main trap
when adding a row (section 3).

There is no separate "apply" step. Left / right writes the value straight into the block.
It is kept when the game is saved, and dropped when the player quits without saving.

The KH1 option names come from the sysmsg strings, and their text was not read (no game data
files here). The names in the table come from what the code does with each value. Read
sysmsg ids `0x126`, `0x12B`, `0x12F` (rows 0 to 2) from `menu/us/sysmsg.bin` to name rows 0
and 1 for certain.

## 2. How the Config menu is built

### 2.1 Pause menu → Config [high]

- Pause menu window update: `0x1402D1690`. It has 8 entries, labelled with sysmsg ids
  `0x69..0x70` (table `0x14052B760`). The visible entry list is ints at `0x142E1FF10`.
  Per entry: window definition `0x14052B780[i]`, opener `0x14052B7C0[i]`.
- **Entry 6 = Config**: opener `0x1402F8CC0` (also `0x1402F8970`). It creates the Config list
  window from definition **`0x140547090`** and the help-line window from definition
  `0x140546E50`. The parent window is kept in `0x142E998F0`.

### 2.2 Menu window framework [high]

- `0x1402E3660(def, layer)` creates a window from a definition. The pool is 24 windows of
  0x118 bytes at `0x142E93160`. The window is the argument of the definition's update
  function, which runs every frame.
- Definition layout (0x30 bytes): `+0x00` update fn, `+0x08` second fn, `+0x10` flags,
  `+0x14`/`+0x16` position offset, `+0x18`/`+0x1A` size, `+0x1C`/`+0x1E` first-row x/y,
  `+0x20` width, **`+0x26` u16 row count (→ window `+0xF6`, `+0xF8`)**, **`+0x28` u16
  visible rows (→ window `+0xFA`)**.
- Window cursor: `0x1402ED230(win)` moves it (up / down / page). `0x1402ED030(win)` returns
  the row: `(win+0xFE + 8 + (win+0xFC & 0xF)) >> 4`. `+0xFE` is the row × 16 in the page,
  `+0xFC` the scroll.
- `0x1402E2E40(win)`: nonzero when this window has focus. Input is processed only then.
- Menu pad state: pointer at `0x142E96320`. `+4` held, `+8` pressed this frame, `+0x10`
  pressed-with-repeat. Left = `0x80000080`, right = `0x20000020` (d-pad bit | stick bit).
  Confirm mask = u32 at `0x140546390` (0x2000), cancel mask = u32 at `0x140546394` (0x4000).
  Read the masks there, don't hard-code them; the PC build may swap them.
- Menu sounds: `0x140177F60(id)`, with 1 = move / toggle, 2 = open, 3 = back, 4 = buzzer.

### 2.3 The Config list window [high]

Definition `0x140547090`: update **`0x1402F7DD0`**, row count 8 at `+0x26` (`0x1405470B6`)
and `+0x28` (`0x1405470B8`). The tables right after it, all u32 and indexed by row:

| Address | Contents |
|---|---|
| `0x1405470C0` | help-line sysmsg id per row: `12A 12E 132 13A 13E 1C6 142 144` |
| `0x1405470E0` | max value per row: `1 1 1 1 1 1 2 0` |
| `0x140547100` | row label sysmsg ids: `126 12B 12F 137 13B 1C3 13F 143` |
| `0x140547120` | mode row value texts: `1CD 140 141` (0 / 1 / 2) |
| `0x140547130` | panel styles for the **selected** row, by value: v0 → (left 7, right 6), v1 → (6, 7) |
| `0x140547140` | panel styles for other rows: v0 → (9, 8), v1 → (8, 9) |

Choice texts per row: (`0x127`,`0x128`), (`0x12C`,`0x12D`), (`0x130`,`0x131`),
(`0x138`,`0x139`), (`0x13D`,`0x13C`), (`0x1C5`,`0x1C4`). They are immediates in the draw
code, not a table. Row 7 is "default settings"; row 6 is the read-only mode.

What `0x1402F7DD0` does each frame:

1. **Input** (only with focus). Cancel closes the window back to `0x142E998F0`. Otherwise
   it moves the cursor, stores the row in **`0x142E998F8`** (the help window reads it), and
   then:
   - Left / right (rows other than 6 and 7: the test is `row − 6 < 2` unsigned) steps the
     value through `0x1402F8EC0` / `0x1402F8F90`, wrapping at the max in `0x1405470E0[row]`,
     and plays sound 1.
   - Confirm. Rows 0, 1, 2 open a pad-diagram sub-window (`menu/md_pad.bin`; windows
     `0x1402F7880`, `0x1402F7A20`, `0x1402F7BC0`, each with 2 choices from `0x1405468B0`,
     and the 12 button callouts `0x1402F76C0`). Rows 3, 4, 5 toggle. Row 6 plays the
     buzzer. Row 7 resets rows 0, 1, 2, 3, 4, 5 to 0 and the inversion floats to −1 / +1.
2. **Draw.** The labels are drawn in a loop over the 8-entry table at `0x140547100`. The
   loop's end is a RIP-relative `lea` to `0x140547120` at `0x1402F835B`, and its start is at
   `0x1402F8285`.
   - Row y = `0x47 + row × pitch`, where pitch = i32 `0x140546050` (26) + i32 `0x140546058`
     (2) = 28. Both are globals shared by every menu.
   - The selected row (if ≤ 6) gets label style 0xB plus four decorations (left / right
     arrows) from `0x1402F0740`. A selected row 7 gets style 1. All others get style 0.
   - Then the value panels of rows 0 to 5, one unrolled call pair each. Then the mode row's
     single wide panel. Then, if row < 6, the pointing-finger cursor (`0x1402ECAA0`), placed
     next to the chosen value.

Drawing calls, all Win64 (for re-use by the DLL):

- Label or value panel: `0x1402D5590(win, style, a, b, x, y, w, h_or_x2, msg_id)`. Label:
  `(win, style, 2, 0, 0x0B, y, 0x8E, 0x1B, id)`. Left choice:
  `(win, style, 2, 2, 0x9F, y, 0xA2, 0xF0, id)`. Right choice:
  `(win, style, 2, 2, 0x147, y, 0xA2, 0x198, id)`. Styles index a 0x14-byte colour table at
  `0x14052B8F0`.
- Selection decorations: `0x1402F0740(win, x, y2, piece)` with
  (−0x40, y2, 0x4B), (0x99, y2, 6), (0x141, y2, 6), (0x1E9, y2, 0x17), where
  y2 = y + i32 `0x140546050` / 2 − 1.
- Finger: `0x1402ECAA0(win, x, yc, 0, 0, frame, 0, 0x80808080, 0x80808080, 0)`.
  Let `c` = u64 at `0x142E91FF0`. Then yc = y + (rowH − (i16 `c+0x152` >> 4)) / 2. Value 0:
  x = 0x139 − (i16 `c+0x150` >> 4), frame 0x10. Value 1: x = 0x14F, frame 0xF. [med for the
  exact args: from the disassembly; Ghidra dropped the stack arguments]

### 2.4 The help line [high]

Window definition `0x140546E50`, update **`0x1402F7D60`**: `0x1402EEB00(0x17)`; then
y = i32 `0x142E96188`; h = a virtual call on the object at `0x142E96180` (slot 1); then
`0x1402EAEA0(win, 0, 0xAE, y, help_id, h, 0x80C87850)` with help_id =
`0x1405470C0[i32 0x142E998F8]`. Row 8 would read `0x1405470E0[0]` = 1 there, which is
sysmsg 1 and the wrong text.

### 2.5 Rows come from code plus static exe tables [high]

There is no data file for the layout. The row count is in the window definition. The
labels, help ids and max values are in `.data` tables (writable). The value storage
mapping and the value panels are hard-coded per row in `0x1402F7DD0`, `0x1402F8EC0` and
`0x1402F8F90`.

## 3. Adding "Camera Style: KH1 / KH2"

### 3.1 Why a plain table extension is not enough [high]

Raising the row count to 9 alone would make the cursor reach row 8. But row 8 would then:
- show no label (the label loop is bounded at 8);
- show no value panels (they are unrolled for rows 0 to 5);
- get left / right handled with `0x1402F8EC0(8)`, which falls to the **mode** (`+0x2C`),
  with the max read from `0x1405470E0[8]` = `0x126`. **Left / right would change the
  difficulty.**
- show the wrong help line.

So we need a wrapper around the window update, plus the row-count change.

### 3.2 Text: menu ids, not handles [high]

This menu does not use the `0x14038B080` handles. Those are generic 32-bit pointer handles
from the PS2 port; the ability table uses them. Every label, choice and help line here is
a **1-based sysmsg id** resolved by **`0x1402CE7C0(id)`**: pointer = u64 `0x142E1B648`
(text base) + u16 at (u64 `0x142E1B650` + (id − 1) × 2). Those two globals point into the
loaded `menu/<lang>/sysmsg.bin` (loader `0x1402CF3C0`, buffer `0x142E1B660`; the language
index is i32 at u64 `0x1421ABA18` + 0x274). Events can repoint the table through
`0x1402D1420` / `0x1402CF720`. Offsets are u16 from the base, so we can't append our strings
to that table.

Plan: replace `0x1402CE7C0` entirely. Ids in a private range (say `0x7F000000 +`) return
pointers to our own KH1-coded strings (`menu.rs`'s `encode`). Every other id is resolved as
above. The function is 21 bytes and starts with a RIP-relative load
(`48 8B 05 89 CE B4 02`, then `FF C9`, `0F B7 04 48`, `48 03 05 74 CE B4 02`, `C3`). So
**never call the trampoline**: `hook::install` would copy a RIP-relative instruction. Give
it the first 7 bytes as `expect` and re-implement the original lookup in Rust. The sysmsg
text uses the same font codes as the remaster's ability strings [med].

### 3.3 Storage [high for both options]

- **Recommended: the DLL's settings file** (`settings.rs`). It is global like a system
  option, and the save stays KH1's own, the same rule `menu.rs` follows for abilities.
- Alternative, per save slot and saved by the game: a u32 in the unused tail of the config
  block, e.g. `0x142E00160 + 0x3FC` (= `0x142E0055C`). 0 = KH1, so new games and existing
  saves default to KH1 because the region is zeroed at new game [med that nothing else uses
  it]. KH1 ignores it, so a save keeps working without the DLL.

## Recommended approach

Four small pieces. The first two make the row exist; the last two make it look right.

1. **Row count.** Once at start-up, write u16 9 at `0x1405470B6` and `0x1405470B8` (the
   Config window definition's row counts; `.data`, already writable). Or write 9 to the
   window's `+0xF6`, `+0xF8`, `+0xFA` at the start of every update call.

2. **Hook the Config update `0x1402F7DD0`** (`fn(win)`). Expect these 15 bytes, three
   register saves, position independent:
   `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18` (5 are enough for `hook.rs`).
   - Before calling the original, if `0x1402E2E40(win) != 0`, read the row with
     `0x1402ED030(win)`.
     - Row 8 with left or right in pad `+8` or `+0x10`, or confirm: flip our value, play
       `0x140177F60(1)`, and **clear the left / right bits** (`0xA00000A0`) in both words
       for the duration of the original call (restore them afterwards). The original then
       sees no left / right on row 8 and never touches the mode.
     - Row 7 with confirm pressed: also reset our value to KH1.
   - Call the original.
   - After it, draw row 8 at y = 0x47 + 8 × pitch (pitch from `0x140546050` +
     `0x140546058`), using the calls in 2.3:
     - the label with our label id: style 0xB plus the four decorations when the row
       (i32 `0x142E998F8`) is 8, otherwise style 0;
     - two choice panels with our "KH1" / "KH2" ids, styles from `0x140547130` (selected)
       or `0x140547140` by value;
     - the finger when selected.

3. **Hook the help line `0x1402F7D60`** (`fn(win)`). Expect
   `48 89 5C 24 08 57 48 83 EC 40` (10 bytes; 18 with `48 8B F9 B9 17 00 00 00`). If
   i32 `0x142E998F8` == 8, redo its three steps (2.4) with our help id; otherwise call the
   original.

4. **Hook `0x1402CE7C0`** as a full replacement (3.2) so our ids resolve to our encoded
   strings: label "Camera Style", choices "KH1" / "KH2", and a help line.

Then back the value with the settings file (or `0x142E0055C`). The camera code reads it.

**Check in game first [low]:** layout. With pitch 28, row 8 spans y ≈ 295..322, but the
help window's definition puts it at y offset 300 (`0x140546E50 +0x16` = `0x12C`, height
`0x54`). The new row will probably overlap the help box. If it does, there are two fixes:
- for the duration of the update call only, lower the shared row gap (i32 `0x140546058`,
  2) or row height (`0x140546050`, 26) and restore them after;
- move the list up through the definition's position offset (`0x140547090 + 0x16`, s16,
  0 now). Check whether the draw helpers add the window position before relying on this.

**Fallback without a new row:** row 6 (Mode) is display-only, and pressing it only buzzes.
The same wrapper could turn that row into Camera Style. That needs no layout change, but it
hides the difficulty from the Config screen. Adding a third choice to an existing row is
worse: the value panels are two fixed slots per row, and rows 0 to 2 open pad-diagram
sub-windows.

**Cheaper label loop (optional) [med]:** the label loop's start (`lea r14` at `0x1402F8285`,
disp32 at `0x1402F8288`) and end (`lea rax` at `0x1402F835B`, disp32 at `0x1402F835E`) are
RIP-relative. Pointing them at a 9-entry copy of the label table in memory within ±2 GB of
the exe (as `hook.rs`'s `alloc_near` allocates) would make the game draw our label and its
selection style itself. A selected row 8 would get style 1, with no arrows. The value panels
and finger would still be ours.
