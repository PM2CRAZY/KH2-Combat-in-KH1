# Cutscene log (t706)

The first step toward the QoL cutscene skipper (the user: "add an auto cutscene skipper in qol", then "make sure the log is toggled in the debug menu so we don't bloat performance"). t706 changes no behaviour: it only writes what KH1 does for a cutscene to `kh1-kh2.log`.

**Switch.** The debug menu's Logs page, Cutscenes & Events (ini `log_cutscenes`, off by default). The hooks go in only with `debug_menu = on` at start and only on the Steam build; with the category off each hook costs one atomic load and a call through to KH1's own function. The log stops after 40,000 lines in a session.

**What it logs** (`win/cutscenelog.rs`):

- Scene prepare (Steam `0x14017CB10 (kind, sub, flag)`): the arguments before and after, KH1's event flag (`0x14298C9A0`) and its scene triple (`0x14233FE84 / 8C / 90`). Checked (decomp).
- Event script starter (`0x1401A38A0 (script file, entry)`): one line for each start: the file, the entry (0 is a room's own script, other entries are events it starts: NPC talks, shops, scenes), KH1's scene flag words (`0x142382590` / `0x142382594`), and (t708) what the script holds read from its context (`0x142382AD0` for entry 0, `0x1423902D0` otherwise): sub-scripts, length, and the distinct 4-byte instructions (class in the last byte, 24-bit operand). Checked (decomp). **Correction (t708):** t706 took the first argument for the context and logged garbage ("counter 53060 op 0xff"); it is the script file, and the context is the fixed object above.
- Movie opener (`0x14011C640 (reader, wide path)`): the file name. Assumption: the second argument is the path and this is the only movie path in KH1; the log shows whether so.

**Hook checks.** Each hook compares its function's first bytes (5 whole, position-independent instructions) and refuses on a mismatch. Checked (exe) for the bytes; not run in game.

**Next.** From a log of a cutscene and a movie: where a cutscene starts and ends in the script, and whether it can be skipped by running the script to its end in KH1's own VM (version after this one, only on the user's go-ahead), then movies.

## t708: what the user's log showed and what changed

The user's log (Traverse Town before and after Guard Armor, "make sure not to skip item shop text, any npcs you talk to ... any cutscene the player voluntarily makes happen"): the scene prepare and movie hooks logged nothing, so KH1 runs these events as scripts inside the room, not as separate scenes (Checked (trace)). The starter lines of t706 were unreadable (above). t708 reads the right object, so each start says what the script is made of. The skipper needs a way to tell a story event from a talk the player started: that comes from this log (Assumption: the instructions differ, e.g. a talk is short and message-only), then from the user's labels of what he did and when (the log's `alive Ns` lines give the clock).

## t709: flag and button watch

The skipper needs the game's own skip action. High-poly dialog cutscenes can be skipped from the pause menu; low-poly dialog scenes and talks cannot and must not be skipped (they can give items). The pause-menu function is not found yet (Checked (decomp): no skip text). With the Cutscenes log on, a `watch:` line is written when KH1's scene flag words, the engine flag word, the event-file player, the NPC context count or the pad buttons change. Test: play a high-poly cutscene, pause, choose Skip Cutscene; then a talk and a low-poly dialog scene with confirm presses. The lines show which words change on each (bit meanings are an Assumption until compared).

## t709 log: player-started interactions (no story cutscene in it)

The user's log covered only things the player starts: Sid in the item shop, Aerith, Leon, the vault with the postcard and the chest that needs Blizzard on the candles (Checked (trace)). Order matched the log: Sid, a room change, then four interactions (the pairing of the last four to names is an Assumption, by order).

- Every one started after a Triangle press, set bit 0x800 of the second scene flag word, and made the engine word read 0xc with Sora unable to act. It ended with the engine word clearing (a 0x2000000 blip first). Checked (trace).
- Sid's shop also set 0xc00, loaded event file 10 and showed engine 0xe: the shop runs through the event-file player, plain talks do not. Checked (trace).
- Bit 0 of the second word (the cutscene-mode bit) never came on in any of them. Checked (trace). That bit is the candidate for telling a story cutscene from a talk (Assumption until a real cutscene is logged).
- Room changes show 0x400 / 0x100 in the first word and a runner state 2 with the engine word at 0x8. These are not interactions. Checked (trace).
- The 0x800 bit stays set after a talk until the next room script resets it, so it marks that a talk started, not that one is running. Checked (trace).

## t709 log: story cutscenes, skipped and played (the signature)

The user's second log had five high-poly dialog cutscenes (Guard Armor aftermath and others), skipped from the pause menu, and one in-game dialog scene played out with confirm presses (Checked (trace); which scene is which is an Assumption from order and pad presses).

- **Skippable scene (high-poly):** the second flag word gets `0x400`, then the engine word gains `0x40000`, then bit 0 of the second word (`0x1`) comes on. Checked (trace), in all five.
- **Skip in the trace:** Start, Down, Confirm. On the next frame the runner word (`0x1423AB2D0`) goes from 2 to 3 (bit 0 set) and the engine word gains `0x400`. About one second later the scene ends, `0x40000` and bit 0 clear and the runner returns to 0. Checked (trace). This matches the earlier decomp lead that `0x1401A22B0` sets bit 0 of that word (Checked (decomp), earlier; not re-read for t709), so that function is the likely game skip action (Assumption until its callers are read).
- **Not skippable (low-poly in-game dialog):** the second word shows only `0x400`, the engine word stays at `0xe`, `0x40000` and bit 0 never appear, and the player confirms through each line. Checked (trace).
- **Player talks and shop:** none of the above bits appear (see the section before).
- Pressing Start without choosing Skip changes nothing in these words (several pauses in one scene). Checked (trace).
- So a skipper can key on `0x40000` plus bit 0 together, which is true only in a skippable scene. Assumption: the same holds for every story scene in the game, not just these six.
