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

## Round 3 (t768): what still held Sora

- **The log and traces (t743).** At every KH2 open the log had "its wait on Sora's motion ... goes on"
  but never an action "done at once", and no "open motion request replaced" line; the watch line showed
  "runner 0x2, engine 0xc" from the frame after the Open until the script ended (about 55 frames);
  the trace (`trace_1793085402_f43568`) has Sora's flags at `0x0130c0c3` from f43411, motion `0xA6`
  (KH1's open motion) on f43411..43413, then idle with the stick held and no movement. Checked (log +
  trace).
- **The treasure script itself** (`exchange/UK_treasure.ev`, the same as `US_treasure.ev`; read in place
  from the user's extracted files, nothing copied here): entry 0 has four contexts. Context 0's first
  sub-script (run at once by the script start, `0x1401A43A0`) opens the windows' layout and ends with
  event functions 113 and 115; its main part starts with 511, then pushes the open's arguments (341..345),
  marks the chest (339), opens the lid, gives, shows the window and ends with 112, 114 and 512. Its
  actions on the player (55, 336, 338, 57, 108, 21/22/26, ...) name the context's actor, which starts at
  0. Checked (files).
- **What those do** (Checked (exe + decomp)): 511 (`0x1401A7540`) sets the runner word's bit 1
  (`0x1423AB2D0`), which makes KH1's game mode "event" (`0x14017EFC0` returns 1); 512 clears it. 113 and
  115 (`0x1401A9A80`, `0x1401A9AA0`) set bits 8 and 4 of the engine word `0x142867364`; bit 4 stops KH1's
  command and pad input for Sora (`0x1402937F0`); 112 and 114 clear them. An action's actor id 0 is the
  party's first actor (`0x140291D30`: ids 0, 1, 2 are the party, `0x1xxxx` an object by its entry, any
  other an actor's own `+4`). t728 matched the records on Sora's `+4`, so none was caught.
- **The fix.** Actions are matched through KH1's own lookup, so the script's actions on Sora go to KH1's
  done queue as before; from a treasure context during a fast open, event functions 511, 512, 112..115 are
  skipped (they take no arguments). The handler's call of the motion request for `0x900A6`
  (`0x1402B59CC`, Checked (exe)) goes to `open_request`: on a fast open (its arguments are already
  written then) nothing is asked, so Sora keeps his run or idle. Assumption: nothing else in the script
  holds him (the trace will say: his flags should stay `0x0000c003`, the watch line should show no
  runner 0x2 / engine 0xc at a KH2 open).

## Round 3 (t769): the room scripts' boxes (Wonderland's evidence)

- **What they are.** The tester's "evidence boxes you find in alice's world" are not chests of KH1's chest
  table: the Open handler never runs for them. They are parts of the room's own script. A context offers a
  Triangle command with event function 454 (`push 0x55, push 1, call 454`); when it is picked, KH1's pick
  (`0x1401A4D00`, from `0x140297CA0`) finds the object's context (the room list `0x142382AD0` / count
  `0x14237F4E4` with entry 0 in use, else the event list `0x1423902D0` / `0x14237F4E8`; the context whose
  `+0` is the object's `+4`), wakes its sub-script 9 (`0x1401A0AF0`: next instruction `+0x190` = the start
  at `+0x90 + 9 * 4`) and keeps the command at `0x142382A38`. Checked (decomp + exe). The sub-script
  checks the command (`push 0x55, 1/6, 3/<skip>`) and runs that command's part. Checked (files).
- **The box's part** (Wonderland `aw05`, the tester's log at f45532: Triangle, then "runner 0x2, engine
  0xc" and "Sora can act: false" until f45665): 424, 113, 115, 321, 378, 593, 511, 113, 115, a wait,
  625, wakes of another context's sub-scripts (Sora's open motion, `push 0xA6, call 108`, and its wait,
  103; the actions 55, 336, 338, 57 on him), 353, 163 / 167, 644 / 258 (`push 0xDF, push 1`: the item,
  added to the stock by 258, `0x1401AC700`), 364, the message (4, 5, 6, 83, 80, 365, 0, 1 with lines
  picked by a variable), 107, 2, then 112, 114, 512. Checked (files + log).
- **Which commands are boxes.** Read across the 236 room files (every script version): the parts that
  take the player (511), give (258, or 583 a reward record) and give him back (512) with no scene,
  choice, shop or menu (88, 126, 222, 268, 278, 392, 628, 629, 673, or a pause-skippable section) are
  command 0x55's in `aw02` and `aw05` (items 0xDF..0xE2, the evidence) and `ew02` (reward records), and
  0x49's in `aw02` (evidence). Traverse Town's 0x72 (`tw01`, four reward records) reads the same way but
  is left KH1's (Assumption: the mailbox, not a box). 0x55 elsewhere (`aw04`: a choice; `pp03`, `lm08`: no
  give; 106 parts the reader can't follow) stays KH1's. Checked (files).
- **KH2 setting (t769).** KH1's pick is hooked: for 0x55 or 0x49, after KH1's own pick, the part KH1 woke
  is read in place (`crate::kh1script::read_box`, unit-tested). A box gets the chest's treatment for the
  contexts of that list until its 512: holds (511 / 512, 112..115) skipped, actions on Sora to KH1's done
  queue (so no open motion), his motion waits passed, message functions in their skip-mode branch; its
  give (258 / 583) shows the notice ("Obtained / Footprints"). The box's own lid and sounds run as KH1
  wrote them. Anything else is KH1's. KH1 setting: nothing installed.
- **Not checked:** in game; the evidence's names come from KH1's item names (`equip::item_name`), and a
  583 record that isn't an item reads "Reward".

## Round 4 (t770): Vanilla | Quick | Skip

- The owner: three values. **Vanilla** is KH1's (nothing installed). **Skip** is t768 / t769 as they were
  (no animation, Sora free at once, the notice). **Quick**: the same, but Sora plays a chest animation first:
  the Open handler's own request for KH1's open motion (`0x900A6`) is made (`open_request` lets it through)
  and KH1's lock (`+0x444` bit 1) is kept; the treasure script's wait on Sora's motion (event function 103,
  `0x1401A96F0`) is KH1's own then, and the frame it goes on the mod clears the lock (`quick_release`). For a
  room script's box the mod makes the same request and sets the same lock at the pick, and lets go at the
  box's wait or its 512. The 8 s window's end lets go too. Assumption: KH1's open motion plays whole with the
  lock on (the t743 trace had it replaced after 3 frames with the lock off).
- **Chest Animation (KH1 | KH2)**, shown only on Quick: KH1 is KH1's open motion. **KH2 has none to port:**
  KH2's Open (`0x1A5`) is a message to the box; the handlers that take it (Steam KH2 `0x140405ED0`, in the
  vtable at `0x1405CC2B0`, and `0x140406020`) play motion `0x98` on the object they belong to, the box's
  set-up (`0x140402200`) picks `0x98` (open) or `0x97` (shut) for the same object, and no player code asks
  Sora for a motion on it. Checked (decomp + exe). Sora's own KH2 motion set (`P_EX100.mset`) has entries
  at `0x97` / `0x98` (`A300`, `A301`), but nothing ties them to chests. Checked (files). Until a KH2 motion
  is chosen, KH2 on this row plays KH1's (logged).

## Round 5 (t771): Quick holds Sora until idle; Chest Animation row off

- The owner: "(c) uncomment the code for the animation switch, we'll find it later. For now make you unable to
  move during the chest animation until you return to the idle animation for the quick setting".
- **KH2 has no chest animation for field Sora** (Checked (files + decomp)): every non-empty motion in
  `P_EX100.mset` is accounted for (movement 0..8, 40, 201..209; item use 9; hit reactions 10..19; guard / recoil
  41..43; the Drive Form change 52, `0x14040CE60`; summon / party calls 137 / 138, `0x140405440`; the summon
  entrance 130, `0x1403DE610`; casts 56..73; attacks 151..197). Field Sora's command handler (vtable entry 39 of
  `PLAYER::VTABLE<SORA>`, `0x140404FD0` -> `0x140405440` -> `0x1403A8B10`) has no case for Open (`0x1A5`) and
  passes it to the chest. Only Atlantica's mermaid Sora (`PLAYER::VTABLE<LMPLAYER>`, entry 39 `0x140405ED0`)
  plays a motion of his own (`0x98`) on Open. The Chest Animation row and the `List::QoL` switch are commented
  out in `config.rs` until one is chosen; `chest_anim` and `chest::anim_kh2` stay.
- **Quick:** the open motion's end (the treasure script's or box's wait on Sora's motion, event function 103,
  going on) no longer lets him go by itself: the next script step that finds his applied request (`+0x164`) on
  KH1's idle (request 0) clears the lock (`quick_release`); a second after the motion's end without idle, or the
  open's 8 s, lets him go anyway (logged). Not checked in game.

## Round 6 (t772): Quick is the treasure script's own hold

- The owner (two traces, t771): "I'm still able to move during the chest animation and when I return to idle I'm
  stuck in the idle state". Checked (trace `trace_1793161449_f2743`): the open motion `0xA6` is applied from
  f2567 to f2707 (140 frames, held on its last pose); Sora moves from f2600 with the stick while `0xA6` is still
  applied (the mod's lock let go early, at its one-second fallback, with KH1 never leaving `0xA6`), and only at
  f2708 does KH1 switch to his run.
- **Why** (Checked (files): `UK_treasure.ev` entry 0): the open motion has no end of its own; the script ends it.
  Context 1 (its actor 0, Sora) holds him (55 at its start; 336 / 338, 57 in sub-script 11 with the wait on his
  motion, 103), then sub-script 12 asks for his idle (`push 0, push 4, call 58`: action 0x11, request 0 with a
  blend of 4, `0x1401A7F80`, Checked (exe)) and unlocks him (56), and sub-script 13 clears the rest (56, 54, 335,
  337). Context 0 waits on sub-script 11 before going on (the `c0x17` wait). t768..t771 sent all of those actions
  to KH1's done queue, so on Quick nothing ever asked for his idle.
- **Quick now** runs the script's actions on Sora as KH1 does (`to_done` only on Skip): KH1's lock from the Open
  handler, the open motion, the wait on it, the idle request and the unlocks, so he is held through the
  animation and freed by KH1 on his idle. Only the window functions (skip mode) and the holds 511 / 512 and 112..115
  are still skipped. A room script's box on Quick: the same (its part plays the motion and frees him); the mod no
  longer asks for the motion itself. The backstop: at the open's end (8 s, or the box's 512) a lock still on
  (`+0x444` bits 0 / 1) is cleared and logged. Not checked in game.

### Round 7 (t773): Quick lets Sora go on idle

- The owner (trace `trace_1793161906_f2177`, log): after the open (slot 83) Sora goes back to idle (slot 0) but
  can't move until the script lets him go. Checked (trace): flags `0x0000c003` before the open; `0x0010c083` on
  the open's frame (f2026); `0x0130c0c3` from f2027 with the open motion `0xA6` applied; idle (request 0) from
  f2108, still `0x0130c0c3`; back to `0x0000c003` only at f2159, with the script's 112 / 114 / 512 (and its
  unlocks 54 / 56 / 335 / 337 in the same second). So about 51 frames on idle, held.
- Checked (exe): KH1's "can act" test (`0x140297870`) is: flags bit 0x4 clear, `+0x88` bit 0x20 clear and
  `+0x444` bit 0 clear. The Open handler sets `+0x444` bit 1 and calls `0x140297F30` (clears flag 0x100000, sets
  0x80, zeroes `+0xC`).
- **Quick now**: at the open the mod keeps Sora's flags and applied request. Once his applied request has left
  that one (the open motion) and comes back to 0 (idle), checked on each of KH1's script calls while the open
  runs, he is let go: `+0x444` bits 0 / 1 off and the hold flags (0x1000000, 0x200000, 0x100000, 0x80, 0x40) put
  back to what they were before the open. From then on the open goes the Skip way (the script's later actions on
  him done at once, its waits on his motion pass), so its late unlocks can't hold him again. The backstop at the
  open's end does the same if he never got there. A room script's box on Quick: the same rule.
- Assumption: those five bits are all that holds him (the log line "Sora is back on idle: let go" prints
  `+0x444`, the flags before / after and `+0x88` to confirm). Not checked in game.

### Round 8 (t775): Quick's notice on vanilla's frame

- The owner: on Quick the "Obtained" notice should come on the frame vanilla's window does, 100 frames into
  the open (the open motion and 20 frames of idle). It showed at the Open itself.
- **Quick now** queues the notice with a due frame: the Open's 60 Hz frame (`takeover::TICKS`) + 100; KH1's
  notice box (`menu`'s drain, which already runs each frame) shows it from then. Skip still shows it at once;
  a room script's box on Quick still at its give.
- Checked (trace `trace_1793161906_f2177`, t772, script timing as vanilla's): the Open at f2025, the open motion
  f2027..f2107, the script's window calls (message functions 0 and 1) at f2129. The 100 is the owner's count.

### Round 9 (t776): KH1's own window, as the evidence boxes'

- The owner: the notice should be exactly the text box the evidence boxes in Wonderland show (their trace
  `trace_1793165977_f7980` and log, a Vanilla box open: slot 83 from f7733, idle from f7814, the window's
  calls 0 / 1 in the second from f7809, its wait 107 until about f7958, no confirm pressed: only L1 / R1 at
  f7886..f7888, so the window closes itself). Checked (trace + log).
- That box is KH1's message window 7. Checked (files): the evidence box's part (`aw05`) sets it up with
  364 (the item), 4 / 5 / 6 / 83 / 80 (window 7's settings), 365 (its text and place), then 0 (open) and 1
  (the line), 107 (waits while it shows), 2 (close). `UK_treasure.ev` sets up the same window 7 with the same
  calls and arguments, only the text ids differ (the chest's 5 / 6 / 13 / 14 .. by kind). So the chest's own
  vanilla window and the evidence box's are the same box.
- **Quick and Skip now** let the treasure script's (and a box's) message functions run as KH1 wrote them,
  instead of their skip-mode branch: KH1 opens window 7 with its own lines and closes it itself, while Sora
  is free (his holds are still skipped / let go as before). The mod's notice in the level-up box is gone
  (`menu`'s notice queue removed). On Quick a chest's window open (function 0 / 177) waits with KH1's own
  "run again next frame" return (4, as function 107 does, Checked (exe)) until 100 frames into the open
  (t775's frame). Skip: the window comes when the script reaches it. Boxes: at their script's own time.
- Assumption: an open message window doesn't stop Sora's own control (in the evidence log he was held by
  the script's holds, which stay skipped). Not checked in game.

### Round 10 (t777): the Dalmatians' window, one line at the top

- The owner: a chest with puppies plays the vanilla puppy text (since t776 the script's own windows show);
  shrink it to the size of the other window so it doesn't block play.
- Checked (files, `UK_treasure.ev`, its text list read as KH1's loader `0x1401A3DA6` reads it): text 4 is the
  Dalmatians' message, two lines ("Found dalmatians." / "Puppy no. A, no. B, and no. C returned home!", the
  numbers KH1's own codes `{0E} 0x10..0x12`, set by event function 475) closing after 90 frames; the
  "Obtained" texts are one line (`{08} {0A}.. {07} {0C} 00` before the words, `{06} 0x3C`: 60 frames). The
  script places text 4 with 365 at (-230, 0), the screen's middle, and its "Obtained" windows at (-220, -150).
  Checked (exe): 365 (`0x1401B7C80`) reads window, text, x, y from the context's stack, measures the text and
  places the window; the script machine's operator 5 negates (`0x1401BA5A0`); the text pointers by id are at
  `0x14237C4E0`, read when a window starts its line.
- **Quick and Skip now**: when the treasure script places text 4, text 4's pointer is our one line in the
  "Obtained" texts' form ("Found dalmatians no. A, B, C.", KH1's number codes, the 60-frame close) and the
  window goes to (-220, -150); KH1 measures, opens, fills in the numbers and closes it as its own. KH1's
  pointer is put back when the script closes that window (function 2) or the open ends. Vanilla: unchanged.
- Not checked in game: the wording fits on one line at the size of an "Obtained" window (Assumption).

### Round 11 (t778): opening from the run

- The owner: on Quick and Skip, open a chest from the run like KH2 (Assumption on their side: only idle works now).
- Checked (exe + traces): KH1 already takes the Open while Sora runs. The chest offers the command while he is in
  state 0 (`0x1402B5A30`), and the Triangle command runs in his states 0, 6 and 0x18 (`0x1402A3768` -> the
  command step `0x1402A3DB0` -> the Open handler); his run is state 0 (every trace: state 0, motion 2, slot 6
  while the stick is held). What ends the run is the Open handler itself: `0x1402B59D7` calls `0x140297F30
  (sora, 0.0)`, which zeroes his speed (`+0xC`), clears flag 0x100000 and sets 0x80.
- **Skip now** keeps his speed: for a fast open that call is skipped (`open_stop`), so he runs on through the
  open as in KH2. **Quick** plays the open motion, so it still stops him as KH1 does. Vanilla unchanged.
- Not checked in game. If Triangle still does nothing while running, a trace of it is needed (it would mean
  another gate than the two above).

### Round 12 (t779): KH2's reach for the Open

- The owner (five traces, Quick): spamming Triangle while running past a chest doesn't open it; it opens only
  when Sora is slow or still against it. Checked (traces `..._f4215`, `_f4631`, `_f4931`): every press at full
  run speed (7..8 a frame) failed, even 7..23 units from where it later opened; the three opens came at speeds
  under ~4 (turning, or stopped against the chest).
- **KH1** (Checked (exe)): the Triangle box is filled each frame by `0x1402948F0`. Sora's touched actor (`+0x98`,
  set by collision, `0x140297C60`) is always asked; then every actor whose record kind (`+0x5F`) is non-zero and
  that `0x140293FC0` finds in range. That range test reads the record's shape `+0x5E` and radius `+0x44`; shapes
  0, 2 and 3 also need the actor in front of Sora (cos > 0; score d + d(1 - cos)/2), shape 1 (flat distance and
  height under the radius) has no facing and scores 0. A chest is kind 3: its touch callback (`0x1402B5A30`)
  offers Open (0x37) when Sora is in state 0 (run included) and the height difference is in (-50, 20). So a
  chest he runs past (beside or behind him) is never offered unless he bumps into it.
- **KH2** (Checked (files)): the box `F_EX030.mdlx` has a type-5 (reaction) shape: a sphere of radius 150, 150
  above its base. Its Open (03system `cmd` 0x1A5) has no distance or facing check (range -1, direction range
  -1), so it is offered whenever that sphere touches Sora's body (radius 35): 185 across, any facing
  (`reaction-commands.md`).
- **Quick and Skip now**: on a chest's first frame (its tick's one-time call `0x1402B5879`, redirected) its
  record gets shape 1 and radius max(its own, 185). KH1's own scan then offers Open from 185 away, any facing,
  running or not. Nothing of ours runs per frame. The log line "chest ...: KH2's reach (offer radius X -> 185,
  shape S -> 1)" shows KH1's own values.
- Not checked: in game; that the record isn't read for anything else (Assumption); a chest set up while
  Vanilla keeps KH1's reach and one set up on Quick / Skip keeps KH2's until the room reloads. Shape 1 scores
  0, so a chest in reach is picked before a Talk in reach (KH2 picks by priority then distance).

### Round 13 (t780): KH2's facing rules, from the KH2 Steam decomp

- **Correction to round 12**: the box's Open in KH2 is not 0x1A5 with no checks. Checked (decomp + files): KH2's
  reaction pass (`0x140407DE0`) walks the box's collision shapes with mask 0x4020; the box (`F_EX030.mdlx`) has
  one type-5 shape, a sphere of radius 150, 150 above its base, whose argument is the command: 0x20 (03system
  `cmd`: execute 32, flags 0x8013, priority 100, `+0x10` -0.7071, `+0x14` 0, `+0x18` 6.1e-17). It is offered
  while that sphere overlaps Sora's body shape (`0x1403C9C00` / `0x1403C94C0`). The score function
  (`0x1404088F0`) greys it (bit 0x40000000, shown, not usable) when Sora's facing · (box - Sora) < -0.7071 (the
  box more than 135 degrees off his facing), when (box - Sora) · (the box's facing turned by 0 + pi) < 6.1e-17
  (Sora behind the box's front half), or when the Land flag (flags bit 1, set) is on and he is in the air.
  (0x1A5, from round 2, is what the box's handlers take; which command the player's press sends to the box
  was not followed.)
- **Quick and Skip now** (t780): the chest's touch callback (`0x1402B5A30`, hooked; KH1 calls it only when its
  scan or a bump asks this chest) checks KH2's two facing rules first; when one fails, KH1's own offer runs and
  slot 3 is then greyed and blocked (bits 3 of `0x140528904` / `0x140528908`, as KH1's scan greys a Talk). KH1's
  own state-0 test stands in for the Land flag. The reach stays round 12's (185, from the box's sphere and
  Sora's body radius 35).
- Assumption: a KH1 chest's front is its yaw's direction ((sin, cos) of `+0x3C`, as Sora's). If chests open
  only from behind in game, the front is the other way.

### Round 14 (t781): no prompt at all (diagnosis)

- The owner (trace `trace_1793170133_f8615`, log): running around the chest at (1190, -1708) while tapping
  Triangle, no prompt appeared. Checked (log): t779 gave all three of the room's chests KH2's reach (KH1's own
  record: radius 150, shape 0; now 185, shape 1), and the chest at (1075, -1228) opened from the run at f7640.
  The traced chest is the Dalmatians one opened at f4147 in the session before (kind 2, contents 5); if that
  open was saved, KH1 offers nothing for it (its touch callback checks the chest is still closed first).
- t781 logs each time KH1 asks a chest (at most one line a second): the chest's place and yaw, Sora's place,
  yaw and state, the distance, KH2's two facing values, whether KH1 offered Open and whether it was greyed.
  No line at all means KH1's scan never asked the chest.

### Round 15 (t782): the chests' facing

- The owner (throne room, three chests: one opened from the run, two never did). Checked (t781 log): KH1 asked all
  three and offered Open each time; KH2's front rule greyed the two at (1190, -1708) and (-225, -1076) every
  time (front -0.45 .. -1.00, Sora south of them), and passed the one at (1075, -1228) (Sora east and north of
  it, front 0.08 .. 0.61). Every chest's yaw `+0x3C` read 0.00, so t780 treated all three as facing +z.
- Checked (exe): KH1's own range test (`0x140293FC0`) takes an actor's facing from its direction vector at
  `+0x30` (dotted with the way to the target), not from `+0x3C`. t782 reads both facings from `+0x30` (flat,
  normalised); a chest with no direction there gets no front rule. The log line now prints both vectors.
- Assumption: a chest's front is its `+0x30` direction. Not checked in game.

### Round 16: KH2's chest animation for Sora (found)

- **Correction to round 2 / t729** ("KH2 has no chest motion for Sora"): it isn't in Sora's own motion set; it is in
  the box's. Checked (files): `obj/F_EX030.mset` (the treasure box, objentry 323 `F_EX030_TR`) has, at entry 252, a
  nested reaction set `AZ10` (BAR type 20) holding motions tagged by character, as KH2's reaction commands do
  (`reaction-commands.md`: the object's script captures Sora into a motion from the object's own set): `0010`
  Sora's body (its 4th slot, 25 088 bytes), `0011` and `0012` his Keyblade's. Each is 30 frames at 30 fps, 60
  frames at 60 Hz (1 s), return frame 30 (no loop).
- Its triggers (Checked (files), not yet named): body: ranges 0..12 id 22 [15], 0..60 id 26 [32], 0..end id 43
  [32], 0..50 id 23 [5], and a frame trigger at 53, id 19 [1]; Keyblade `0011`: ranges 10..23 id 12 [28], and
  frame triggers 34 and 47 (id 8, [4, 500]).
- Checked (decomp): the box's handler (`0x140405ED0` / `0x140406020`) takes message 0x1A5, copies its prize and
  plays the box's own lid motion 0x98 with an 8-frame blend; the Open offered to the player is command 0x20, the
  argument of the box's type-5 shape. Not followed yet: the box's script (BDX) that captures Sora into `AZ10`,
  and whether Sora is held for all 60 frames.
- Open questions for the owner: Skip was built on "KH2 plays no animation"; KH2 does play this 1 s motion. And
  the Chest Animation row (KH1 | KH2, commented out since t771) can now be built from this motion.

### Round 17 (t783): Quick's Chest Animation row, KH2's motion

- The owner's answers: Skip stays a no-animation setting ((a)); build the Chest Animation row from KH2's motion.
- **Loading** (`pose::add_chest_motion`): with KH2 Sora's files, the box `F_EX030` is read as an object
  (`kh2_data` `load_enemy`), its capture slot 252's `0010` / `0011` / `0012` (`EnemyData::capture`, the same
  path `kh2-bevy`'s reaction commands use) added to his moveset and rig as motion 1252 (`pose::CHEST_MOTION`:
  1000 + the box's slot). Its two files go into the slot cache's manifest.
- **As a KH1 slot** (`kh2slots::CHEST`): KH1's open request 0xA6 (slot 83, KH1's open motion, its template)
  gets a slot of ours, KH2's 60-frame motion (KH1's own is 81), baked into a KH1 motion with `kh1_bake`
  (else KH1's retimed and KH2's pose drawn over it), KH1's sounds kept with KH2's frame triggers added.
  `redirect` points 0xA6 at it only while `chest::kh2_animation()` (Chest Opening Quick and the row at
  KH2); both setters refresh the requests at once.
- **Config**: QoL > Chest Animation (KH1 | KH2) is back, shown only with Chest Opening on Quick.
- **What still runs as before**: the Open handler asks for 0xA6; KH1 plays our slot; the treasure script's
  wait on his motion ends at its end and asks for his idle; Sora is let go on idle (t773) and the item window
  comes 100 frames into the open (t775).
- **Not done, open**: the motion's turn-to-target range (id 22, frames 0..12, 15 a frame: KH2 turns Sora to the
  box) has no KH1 counterpart here, so he opens facing the way he was. Range ids 26, 43, 23 and frame id 19 are
  unnamed. KH2's own hold length (the box's script) not followed; Sora is held by KH1's script until the
  motion ends. Not checked in game.

## Round 5 (t795): items picked up off the ground (QoL > Item Pickup)

- **What it is.** The owner: picking an item up off the ground holds Sora until the window is gone. These are not
  chests nor the evidence boxes' 454 commands: the Triangle command's kind 4 / 5 (`0x1402A3DB0` and
  `0x1402A4150`) queues event 0 / 4 on the object (`0x14028DD80`), and KH1's event dispatch (`0x1401A1A10`,
  run by `0x14028E550`) wakes the object's context's sub-script 5 (event 0) or 3 (event 4). Checked (decomp).
- **The part** (Destiny Islands `di01`, the owner's t794 log at f3289): a guard on a variable, 511, 113, 115, a
  wake of Sora's context (his pick-up motion), 644, 258 (`push item, push 1`), 364, window 7 (4 / 5 / 6 / 83 /
  80 / 3, then 0 and 1 with a line picked by a variable), 353 0x1F, 107, 2, a wake, 112, 114, 512. Checked
  (files + log).
- **Where they are.** Across the 239 room files, whole sub-scripts that read 511, a give (258 / 583 / 458 / 444
  / 570), window 7 only and 512, with no scene, choice, shop or menu: `di01` / `di03` (Destiny Islands, items
  0xC0..0xC7), `tw*` (0xD3), `pc05` / `pc06` (0xB2..0xBF), `tz13` (0xD9..0xDE), `lm*`, `pp10`, `po*`, `pi37`,
  `nm01` / `nm11`, `he01` / `he04`, `ew22`, `aw*`. Checked (files; a byte scan, not every one tied to its
  command; the world of each room code other than `di` is an Assumption).
- **Skip (t795).** KH1's dispatch is hooked: for events 0 and 4, after KH1's own wake, the woken sub-script is
  read in place (`crate::kh1script::read_pickup`, unit-tested). A pickup gets the box's treatment for that
  context list until its 512: the holds (511 / 512 / 112..115) skipped from the pickup's own context only,
  actions on Sora to KH1's done queue (no pick-up motion), his motion waits passed. Its give, window 7, sound
  and flags run as KH1 wrote them. Anything else is KH1's. Vanilla: nothing changes (the hook is installed the
  first time the row is set to Skip and then passes everything to KH1).
- **Not checked:** in game. Assumption: the window doesn't hold Sora (as with chests). If the part's guard
  jumps past the give, the mode ends at the open's 8-second time instead of a 512.

## Round 6 (t801): pickups that wait, touches, a log for every way out

- The owner's t800 log: Destiny Islands' pickups still held Sora, and no Item Pickup line showed at all.
- **Why one can be missed** (Checked (decomp)): KH1's wake `0x1401A0AF0` pops the context and a priority (0..7).
  If the priority is at least the context's current one (`+0x32C`), the next instruction (`+0x190`) becomes the
  sub-script's start at once; otherwise the start waits in `+0x30C + priority * 4` and runs when that level ends.
  t795 checked only the next instruction. t801 also takes the waiting slot.
- **Events read** (Checked (decomp)): 0 (sub-script 5, priority 3) and 4 (3, priority 3) from the Triangle command
  step; 1 (6, priority 2) from contact (`0x140297E80`); 3 (7, priority 4) from `0x1402B0610`. Event 2 (hits) is not.
- **Log:** every way out for events 0, 3 and 4 says why (Cutscenes). Not checked: which event the island's items use.

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
