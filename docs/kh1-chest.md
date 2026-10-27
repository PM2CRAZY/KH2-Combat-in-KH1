# Chest Opening (KH1 | KH2)

The QoL row Chest Opening (ini `chest_style = kh1 | kh2`, KH2 by default since round 2; it was Combat > Tweaks, KH1 by default, in t724) asked by the
tester Mr Bugs (thread kh2-chest): "quickly open and move from the chest opening animation like in
kh2 ... open the chest, move then the pop up for the item plays at the top of the game letting you
know what you got". Code: `crates/kh1-hook/src/win/chest.rs`.

## How KH1 opens a chest (Steam)

- **The Open command.** A chest's touch callback (`0x1402B5A30`) offers command `0x37` in the
  Triangle box when the chest is not opened yet (`0x1402850F0 (chest id)`, the opened bits at save
  `+0x5CC`) and Sora's height is within -50..20 of it; in battle it takes the offer back
  (`0x142867364` bit 0). Checked (decomp).
- **The handler `0x1402B58E0 (sora, chest)`** (from the command step `0x1402A3DB0`, case `0x37`)
  reads the chest's u16 in KH1's chest table `0x140528D60` (by the chest id at its object `+0x48`):
  bits 1..3 the kind, bits 4.. the contents. It gives the contents itself, before any script:
  - kind 0: an item, KH1's stock add `0x140284E60 (item, 0xC)` (0xFF back when the stock is full;
    then the chest is not marked opened);
  - kind 1: `0x14020D3F0` (not identified);
  - kind 3: munny, the contents x 10, through `0x1402E7E10`;
  - kind 7: a reward record (`0x14028F9A0`); an item record becomes kind 0 with its item;
  - any other kind (5: an event chest) gives nothing here.

  Then it marks the chest opened (`0x140285800 (id, 1)`) and counts it (`0x14028C660 (0x10)`),
  stores the open's arguments at `0x142DB7B28` (the chest's handle, its id, the kind `+8`, the
  item or amount `+0xC`, "stock full" `+0x10`), sets Sora's `+0x444` bit 1 (his control step is
  skipped while it is: locked), asks for his open motion (`0x14029F980 (sora, 0x900A6, 0)`), sets
  `+0xC` to 0 (`0x140297F30`) and queues event 5 on the chest (`0x14028DD80`, queue `0x142BA2080`).
  Checked (decomp + exe).
- **Event 5** (`0x1401A1A10`, run from `0x14028E550`) copies the five arguments to `0x142382700`..
  and starts KH1's treasure script, `exchange/<lang>_treasure.ev` (loaded at `0x1423BBCB0` by
  `0x1401A42C0`; on the English Steam build `UK_treasure.ev` in `kh1_first`), with
  `0x1401A43A0 (script, entry)`: entry 0 for every kind but 5, whose entry is its contents. That
  script runs in its own contexts (`0x1423BF0B0`, `0x360` each, count `0x1423AB2DC`; the script in
  use at `0x1423BBBE0`), stepped by the task `0x1401A2CF0`. Checked (decomp).
- **What the treasure script does** is not read yet (the file was out of reach in this session:
  `kh1_first.pkg` is over the transfer limit). Assumption: it opens the lid, shows the "obtained"
  message window, waits for it to close, and releases Sora with KH1's actor action kind 0xF
  (`0x14028E6E0`'s case 0xF clears `+0x444` bit 1; case 0x10 sets it).
- **Message windows** (Checked (decomp)): four slots, state bytes at `0x14237F4F0` (0 free, 1
  opening, 2 showing, 3 closing, 4 done), the window id per slot at `0x142380520`, lines per id at
  `0x14237F500` (`0x200` apart) with counts at `0x142380500`, layouts per id at `0x1423D4560`
  (`0xD0` each), window objects at `0x1423D4220`. Event functions: 0 (`0x1401B93F0`) and 177
  (`0x1401B9170`) open a window, 1 (`0x1401B8DD0`) adds a line, 605 (`0x1401B8AE0`) adds an item or
  ability line from a reward record, 2 (`0x1401B8100`) closes one (nothing when it isn't open), 33
  (`0x1401BA270`) waits until every open window but id 7 is closed, 107 (`0x1401BA2C0`) waits while
  a window id is opening or showing. In skip mode (`0x1423AB2D0` bit 0, mirrored at `0x1423404EC`)
  0, 1, 177 and 605 pop their arguments (1, 2, 1, 2) and open nothing; with no window open the
  waits pass. The event function table is `0x1404FA870` (8-byte pointers by number).

## What the KH2 setting does (first step, t-version of this row)

For a plain chest only: kind 0 or 3 after KH1's handler ran, the stock not full.

- KH1's handler runs whole (the item or munny given by KH1's own functions, the chest marked, the
  open motion asked for, the treasure script started).
- Then Sora's `+0x444` bit 1 is cleared: KH1's own control step runs him at once.
- For 8 s after the open (`FAST_MS`), event functions 0, 1, 177 and 605 called from a treasure
  context (the script in use is the treasure file and the context is one of its contexts) take
  KH1's own skip-mode branch: the treasure script's "obtained" window never opens and its waits on
  it pass. The rest of the script (lid, sounds, effects, its release) runs as KH1 wrote it. Their
  four table entries point at wrappers of ours (installed only once the row is first set to KH2).
- "Obtained" and the item's name (or "N Munny") show in KH1's level-up notice box
  (`menu::queue_notice`, the box the mod's "New Ability" notices use), which needs no button.

Anything else stays KH1's: event chests (kind 5), reward records that aren't items, kind 1, a full
stock, Trinity chests (they are scripts of the room, not this handler), any other script in those
contexts.

## Not checked (Assumption)

- KH2's own timings: KH2's treasure-box code (its BDX trap `trap_treasure_get`) wasn't in the
  KH2 decomp chunks at hand. From memory of KH2: no lock on the player, the item in the top-left
  information bar. Not checked against the KH2 exe.
- That the treasure script's only blocking step is its message window, and that it queues nothing
  on Sora that pulls him back (a motion, a turn) once he has walked off. The log lines say what was
  skipped.
- Where KH1's level-up box shows on screen (it is KH1's own; the tester asked for the top).
- Whether KH1's "obtained" jingle belongs to the window (then it is gone) or to the script (kept).
- Archipelago's patched `treasure.ev`: its windows are skipped the same way; the notice names what
  KH1's chest table gave.
