# KH2's command menu in KH1

The user: "in the main build, we had code that was trying to put in quirks of kh2's command menu, I
want to gut the system and make sure that its 1:1 and any features that are different between the
two (remembering commands on sub menus, locking your command until your actionable etc) are in the
command menu config."

Built in t751..t756 (see "What was built" at the end). Steam addresses. Sources: the Steam KH2 decomp, the Steam KH2 exe's
data (button tables), KH2's `03system.bin` `cmd` table and `msg/us/sys.bar` (names), the Steam KH1
decomp, and `docs/kh1-drive.md` (KH1's menu structs). Each note is tagged.

## KH2's command menu

**The object.** `YS::PLAYER_COMMAND` (Sora's is the global at `0x142A10620`, vtable
`0x1405C4C68`: init `0x1403F2990`, rebuild `0x1403F29E0`, input `0x1403F2470`, run
`0x1403F22E0`). It holds 19 menus of 0x80 bytes from `+0x28` (menu k at `+0x28 + k*0x80`); a
menu's `+0` is its number, `+0x10` its row count, rows of 12 bytes from `+0x14` (`+2` command id,
`+5` status: under 4 selectable, 5 greyed), `+0x74` the cursor row and `+0x78` the cursor's
command id. A stack of open menus from `+0xAF0`, its top pointer at `+0x9A8`. The current root
page at `+0xBD4` (0 or 6). Checked (decomp).

**Menus** (rebuilt every frame by `0x1403F29E0`). Checked (decomp + files):

| menu | what |
|---|---|
| 0 | page 1: Attack, Magic (blank when no spells), Items, Drive (Revert in a form; Magic is Limit in Limit Form) |
| 1 | Magic list |
| 2 | Items list (duplicates stacked) |
| 3 | Drive list (forms 1..6; a form not ready is greyed, one not found is left out) |
| 4 | party target list (Cure, items) |
| 5 | L1 shortcuts (4 rows, each tied to a button) |
| 6 | page 2: Attack, Summon (`0x48`, opens 7), Party (`0x49`, opens 8), Limit (`0x51`, opens 9); a row not available is the blank command `0x68` |
| 7 | Summon list |
| 8 | Party list (the two party members) |
| 12 | Limit Form's Limit list |

Command names from `cmd` + `sys.bar`: 1 Attack, 2 Magic (sub 1), 3 Items (sub 2), 4 Drive (sub
3), 5 Revert, 0x48 Summon (sub 7), 0x49 Party (sub 8), 0x51 Limit (sub 9), 0x2C3 Limit (sub 12).
Checked (files).

**Buttons** (KH2's field button table, `0x1405C34F0`, read from the exe). Checked (exe):
d-pad up / down move the cursor, d-pad right opens the cursor row's submenu, d-pad left is its own
bit, confirm and cancel are the swap-aware decide / back bits; with L2 held the right stick is the
d-pad and R3 is confirm.

**Input step** (`0x1403F2470`, then the menu's own step `0x1403B11E0`). Checked (decomp):
- Up / down: wrap at the ends, sound 1. Pressed only, no hold-to-repeat.
- D-pad right: opens the row's submenu if the row is selectable (sound 3).
- D-pad left at the top level: flips page 1 / page 2, the cursor keeps its row number, sound 0x14
  (only while the story allows the second page and no situation commands replace the menu).
- D-pad left in a submenu: backs out one level, sound 4. The parent's cursor is kept.
- Cancel in a submenu: backs out one level, sound 4; if that lands on the top level, the top
  cursor goes back to Attack. Cancel at the top level is passed to Sora (the menu sets a flag).
- Confirm on a greyed row: error sound 5.
- L1 held with shortcuts: the shortcut menu replaces the top level; when L1 is let go the top level
  comes back with its cursor on Attack.
- A submenu that becomes empty closes back to the top level (cursors reset, sound 4).

**Remembering the cursor.** Checked (decomp):
- The cursor is kept by command id, not row: when a list is rebuilt (a spell learned, an item used
  up) the cursor stays on the same command; if it is gone, row 0.
- After a command is run, every open submenu's cursor and the top cursor go back to row 0
  (`0x1403B21D0`). Backing out without running keeps the submenu's cursor.

**Locking the command until Sora can act.** Checked (decomp):
- Confirm asks `0x1403B6950` / `0x1403BA070` whether Sora can take this command now (no current
  action, or the action is in a cancel window, and his recovery timer at `+0xB94` is out; some
  commands always pass). If not, nothing happens: no sound, the submenu stays open, nothing is
  queued. The press must be made again once he can act.
- Rows are greyed only by availability (MP, items, form, story), never by what Sora is doing, and
  an open submenu is never closed because he started an action.

**Not found yet:** where KH2's player reads the menu's per-frame flags (jump pressed, cancel at the
top level, guard pressed, guard pressed in a submenu), so whether KH2 jumps on the cancel press
that backs out of a submenu is not confirmed. Assumption until found: it doesn't (the flag for
cancel is only set at the top level). Also not traced: what confirming a spell with too little MP
does (its row status is 2, still under 4), and what Party does when run.

## KH1's command menu (for comparison)

Steam: top menu struct `0x140528900` (0x38 bytes each, k = 0..6), chain `0x142D539F0`, level
pointer `0x142D53A50`, navigation `0x1402931B0`, confirm `0x140293410`, run `0x140293480`, fill
`0x140294220`. Checked (decomp), with `docs/kh1-drive.md`:
- Up / down wrap, sound 1. D-pad right opens a submenu (not on row 4), sound 6.
- D-pad left at the top level: cursor to Attack (no second page). In a submenu: back out, sound 3,
  parent's cursor kept.
- Cancel in a submenu: back out, sound 3, and the parent's cursor goes to row 0 (any parent, not
  only the top level). Cancel at the top level is Sora's jump.
- Row 4 is not a real entry: it is the magic list's last entry (Summon).
- The cursor is kept by row number; a list that shrinks clamps it.
- After a command is run, the chain goes back to the top; the top cursor goes back to Attack
  (unless the command's record has flag 0x10); submenu cursors are kept, so the Magic list reopens
  on the last spell. Which records carry 0x10: not checked.
- Each frame the top level's Attack bit and the magic and item lists' entries are greyed and
  blocked by Sora's movement state (the state table's bits 0x10 / 0x20 / 0x40, `0x140294220` /
  `0x1402944C0`). **Correction (t755):** an open list is closed only when its top row becomes
  blocked (`0x1402931B0`), which the state doesn't do: KH1 greys the list's entries while he is
  busy, it doesn't close Magic. Confirming a blocked entry gives the "can't" sound.
- Run refuses while Sora can't act (one exception: Fire chained in its own cast), the menu stays.
- Not checked: what KH1 does with the top cursor when L1 is let go.

## The differences

| | KH1 | KH2 |
|---|---|---|
| Top level | Attack, Magic, Items, Summon (row 4 borrowed from the Magic list) | page 1 Attack, Magic, Items, Drive; page 2 Attack, Summon, Party, Limit |
| D-pad left, top level | cursor to Attack | flip page, cursor keeps its row |
| Cancel back to the top | parent cursor to row 0 (any level) | top cursor to Attack (only when it lands on the top) |
| After a command | top to Attack, submenus keep their cursor | everything to row 0 |
| List changes | cursor by row (clamped) | cursor follows the command |
| Busy Sora | spells and items grey and are blocked ("can't" sound); an unblocked command is taken and run or dropped | rows stay lit, the list stays, confirm does nothing until he can act |
| Empty submenu | (closes when the parent row blocks) | closes to the top, sound 4 |
| L2 + right stick | (our L2 Stick Menu) | always on; L2 + R3 confirms |
| Sounds | 1 move, 6 open, 3 back, 4 can't | 1 move, 3 open, 4 back, 5 error, 0x14 page flip (KH2's own bank) |

## What we have now (from main), to be taken out

- `drive.rs`: d-pad left flips to a "page 2" that is KH1's menu with the Summon row turned into
  Drive, by rewriting the magic list's last entry and the summon list every frame; a confirm-swallow
  after a form; Revert on the Drive row. This is the reverse of KH2 (KH2's Drive is on page 1,
  Summon on page 2) and not a real row.
- `combat.rs` `run_entry` / `spell_not_yet`: the "stay in the menu until Sora can act" rule asks
  KH2's controller (`takeover::can_cast`, `can_take_command`) and KH1's cut windows, with special
  cases for Valor, items and summons in the air. Not native.
- `takeover.rs` / `probe.rs` / `shortcuts::submenu_open`: pad masks so Circle and Square do the
  right thing with a submenu open while KH2's controller reads the pad.
- Main only: `menujump.rs` (the jump-hold after backing out).

## Decisions (the user, round 1)

- Page 2's Party and Limit rows: "(a) we'll add them later" (blank as KH2 shows them while not
  available; the systems come later).
- Drive and Summon: "for this situation we want a toggle where you can switch summons or drive
  either on page 1 or 2". A Command Menu row: Drive on page 1 and Summon on page 2 (KH2's), or
  Summon on page 1 and Drive on page 2.

## Decisions (the user, round 2)

- Page 2 with Menu Layout KH1: "(b) but you can add a config in the menu for vanilla kh1
  behavior". Page 2 exists on both layouts (d-pad left at the top level flips). A Command Menu row
  for d-pad left: Page Flip, or KH1 (vanilla: cursor to Attack, no page 2).
- Sounds: "(b) but for when kh1 doesn't have a sound that covers functionality thats in kh2 then we
  add another sound e.g the sound for when you flip to page 2". KH1's menu sounds always (move 1,
  open 6, back 3, can't 4); where KH1 has none, KH2's sound is added to KH1's sound system: the page
  flip (KH2's system sound 0x14). No Menu Sounds row.

## Command Menu rows (built; Menu Layout was folded into Drive Page, t752)

| Row | KH1 | KH2 |
|---|---|---|
| Drive Page | Page 2: Summon in row 4, Drive on page 2 | Page 1: Drive in row 4, Summon on page 2 |
| D-Pad Left | cursor to Attack, no page 2 | flips page 1 / 2, cursor keeps its row |
| Cursor Memory | KH1's (submenus keep their cursor) | KH2's (all to row 0 after a command, cursor follows the command) |
| Command Lock | KH1's (rows grey and submenus close while busy) | KH2's (rows stay lit, confirm does nothing until Sora can act) |
| L2 Stick Menu | (existing row) | plus L2 + R3 confirms |

## What was built (t751..t756)

All in `crates/kh1-hook/src/win/cmdmenu.rs` unless named. Steam only; nothing here was run in game
yet.

- **t751, the rows.** Command Menu list: Drive Page (Page 1 | Page 2), D-Pad Left, Cursor Memory,
  Command Lock (KH1 | KH2), then L2 Stick Menu. Ini `drive_page`, `dpad_left`, `cursor_memory`,
  `command_lock`; KH2 and Drive on page 1 by default.
- **t752, the pages.** D-pad left at the top level flips the page (KH1's own d-pad left stays with
  D-Pad Left KH1). After KH1's own top fill (`0x140294220`, hooked) page 2's rows are written over
  KH1's top level: Attack, Summon (as KH1's row 4 has it) or Drive, then Party and Limit as KH1's
  empty command. Drive on page 1 stays KH1's own row 4 (the magic list's hidden last entry): the
  plan had KH1's fill, draw and step take a real fourth entry, but KH1's row 4 already is its own
  engine's row, so nothing changes in how it plays and less of KH1 is patched. `drive.rs` lost its
  own page; it fills Drive's row and list on whichever page shows Drive.
- **t753, the page flip sound.** KH2's system sound 0x14 (se000's 20) cut for KH1's engine with the
  other KH2 sounds and played on the flip; KH1's cursor sound until it's loaded. Every other menu
  sound is KH1's.
- **t754, Cursor Memory.** KH2: every open list and the top level back to row 0 after a command; a
  cursor follows its command when a list is rebuilt; cancel two levels deep keeps the parent's
  cursor.
- **t755, Command Lock.** KH2: KH1's state greying lifted in the battle states (the state's "may
  cast, may use items" bits set around KH1's top fill), so only availability greys; a spell, item
  or summon is refused (no sound, the list stays) while KH1's can-act and KH1's combo window are
  both closed. Attack is left to KH1's combo input. Gutted: the KH2-controller sections of
  `combat::spell_not_yet`.
- **t756, L2 + R3.** With L2 Stick Menu on, R3 pressed with L2 held is KH1's confirm (R3 kept from
  KH1 then).

Still open (no answer needed):
- Where KH2's player reads the menu's per-frame jump / guard flags (whether KH2 jumps on the cancel
  that backs out of a list).
- KH1's draw of the empty command 0x42 on page 2's Party and Limit rows (what KH1 shows for its own
  empty rows).
- KH1's Attack row still greys by state with Command Lock KH2 (KH2's Attack doesn't); left as KH1's
  for now, since lifting it would let KH1's attack step start from states it doesn't expect.
- The takeover pad masks for an open list (`takeover`, `probe`) are left: they only act while KH2's
  controller has Sora (the fallback).
