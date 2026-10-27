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

## Round 2: what still held Sora (t725 trace), and the release

- **The trace.** The tester's three Select + Triangle traces (t725, the row on KH2): on the frame after
  the Open, Sora's actor flags go from `0x0000c003` to `0x0130c0c3` and KH1's motion request to `0xA6`
  (the open motion, 81 frames); he stays put with the stick held until 132 frames after the Open, when
  the flags drop back to `0x0000c003` and he walks off. Checked (trace). The bits are KH1's actor
  actions 0x24 (`0x1000000`, the script hold the mod's takeover reads as "KH1's script moves him"),
  0x26 (`0x200000`) and 0x2B (`0x40`), and their clears 0x23 / 0x25 / 0x2A (`0x14028E6E0`, Checked
  (decomp)): the treasure script queues them on Sora; action 0x10 sets the `+0x444` lock t724 cleared,
  so the script could lock him again. Assumption: which of them the script queues (its file is in
  `kh1_first.pkg`, still out of reach this session); the flags say at least 0x24, 0x26, 0x2B.
- **KH1's actions.** Event functions queue actions with `0x1401A5030 (record)` on a ring of 256
  records of `0x20` at `0x1423CFE10` (write index `0x1423CFE08`, read `0x1423CFDFC`): `+0` kind |
  argument << 8, `+4` the actor's id (most functions push the context's own, `*ctx`), the rest by kind.
  `0x14028E6E0` runs them (actor by id, `0x140291D30`: actor `+4`); every kind but 0, 1 and 0x28 needs
  an actor (table `0x140528368`), and one whose actor isn't found goes straight into the done queue
  (`0x1401A4F20`, ring at `0x1423D1E10`). The script runner (`0x1401A2CF0`) clears a context's move /
  turn waits (`+0x33C` bit 0 for kind 2, bit 1 for kinds 4 / 0x15 / 0x16) from the done records with
  its actor. Event function 103 (`0x1401A96F0`) waits for the context's actor's (`ctx + 8`) motion to
  end (`0x14029EFC0`). Checked (decomp + exe).
- **The script machine's class table** is `0x1404E5DB0` (8-byte handlers by class, loaded by `lea` in
  `0x1401A3517`, `0x1401A4658`, `0x1401A4917`); class 0x18's handler `0x1401B3530 (ctx, number)` jumps
  to the event table's entry. Checked (exe).
- **The release (KH2 setting).** At a fast open, our handler takes class 0x18's place in that table
  (one data word; KH1's is put back at the first call after the open's 8 s). From a treasure context it
  calls KH1's function, then moves the actions that call queued on Sora into the done queue with KH1's
  own push (the others stay in order), and it lets function 103 on Sora go on at once. So the script
  runs whole (lid, sounds, give, flags) and its own waits end as if Sora had done what it asked, but he
  is never held, locked, turned or animated by it. Off (KH1): nothing installed. Lines: "Chest Opening:
  treasure script: action 0x.. (argument ..) on Sora from function N done at once", "... its wait on
  Sora's motion (function 103 ...) goes on", "the open's time is over (...)" (Cutscenes & Events log).

## KH2's own chest opening (round 2: what Sora does)

- **KH2's Open** is command `0x1A5` in 03system's `cmd` table: execute 421 (its own id), action 0,
  priority 100, no camera (Checked (files), the user's `03system.bin`). Reaction commands that play
  a reaction on the player use execute 81 (119 entries, e.g. `0x1A7`); Assumption: that 81 is the
  player-reaction path (not traced in the exe). The treasure box takes `0x1A5` as a message: its
  handlers (Steam KH2 `0x140405ED0`, `0x140406020`) copy the box's prize data, play the box's own
  motion `0x98` (the lid opening) with an 8-frame blend (`0x1403C86A0 (motion, 8.0, 0)`) and set its
  state 6; set-up (`0x140402200`, `0x1404024E0`) picks `0x98` (opened) or `0x97` (closed). The item is
  given by `trap_treasure_get` (BDX trap 1 / 362, `0x1404317F0`, its entry at `0x140756A10` of the 16-byte trap table).
  KH2's player has no chest action: the action classes the exe's RTTI names (`ACTION_IDLE`,
  `ACTION_JUMP`, `ACTION_GLIDE`, `ACTION_PRIZEBOX`, ...) have none for a chest (Checked (exe)).
  So in KH2 Sora plays no open motion and isn't held: nothing to port for his side.
- **KH2 setting.** KH1's handler asks for Sora's open motion (`0x900A6`); right after it, that pending
  request is replaced by the motion Sora had before the Open (`+0x164`), with KH1's own request
  `0x14029F980` (its third argument a float, XMM2; Checked (exe)). Line: "Chest Opening: KH1's open
  motion request replaced by Sora's own motion 0x..". Assumption: re-asking for the motion already
  playing doesn't restart it (`0x14029F7C0` compares the new motion with the one on).

## Gummi and Dalmatian chests (round 2)

- KH1's chest table (`0x140528D60`, Steam exe) by kind over its first 600 entries: 425 kind 0 (items),
  81 kind 7 (reward records), 45 kind 1, 33 kind 2, 13 kind 5, 2 kind 4, 1 kind 6, no kind 3.
  Checked (exe). Kind 1's contents run 0x02..0x8A and the handler gives them with `0x14020D3F0`, the
  gummi hangar's add (it caps a block at its record's `+0xB` count, 1 for 0x47..0x76) — gummi blocks.
  Kind 2's contents are 1..0x21 in order, 33 entries: the Dalmatians (99 puppies, three a chest); the
  handler gives nothing for kind 2, so the treasure script does. Checked (decomp + exe) for the
  handler; that kind 2 is the Dalmatians is from the count and order (Assumption, strong).
- t724 sent kinds 1 and 2 to KH1's open. Round 2 takes them the KH2 way too: the handler and the
  treasure script run as before (so the Dalmatians' give, whatever function the script uses, still
  runs), with only the windows skipped and the actions on Sora released. The notice reads "Gummi
  Block" (Assumption: the gummi's own name isn't read yet) or "Dalmatians 01, 02, 03" (Assumption:
  set n is puppies 3n-2 .. 3n). A full gummi hangar stays KH1's, as a full item stock does. Kinds 4,
  5, 6 and non-item reward records stay KH1's.

## The notice (round 2, question 1: "(b) the top of the screen like KH2's info bar")

- KH1's level-up boxes, which the notice uses, are updated by `0x140272F10` (Steam; boxes at
  `0x14283B390`, `0xBA0` each, 5 per member, members `0x3A20` apart, counts at `0x14283B380`): each
  column starts at `+0x10` from the top and the boxes stack downward (`+0x16` a line), one column per
  party member from the left. Checked (exe). So the notice already sits in the top-left corner, where
  KH2's information bar is.
- Not done: KH2's own bar (its art and its slide / fade timing). That is a new HUD element drawn with
  KH1's sprite calls from KH2's field 2D layout, and it needs a reader of KH2's 2D layout files and
  their images on this branch (main's Drive gauge has one; this branch may not take main's code). Its
  timing in KH2's exe isn't traced yet (the chest's give is `trap_treasure_get` -> `0x1401595B0`).

## Not checked (Assumption)

- (t724, now answered) KH2's own chest: see "KH2's own chest opening" above (checked in files and
  exe for the command and the box; the player side from the absence of a chest action).
- (t724, replaced) "the treasure script's only blocking step is its window": wrong, it also held Sora
  through actions on him (round 2's trace); those take KH1's no-actor path now. Still not read: the
  script file itself (which actions, in which order).
- Where KH1's level-up box shows: top left (round 2, Checked (exe)).
- Whether KH1's "obtained" jingle belongs to the window (then it is gone) or to the script (kept).
- Archipelago's patched `treasure.ev`: its windows are skipped the same way; the notice names what
  KH1's chest table gave.
