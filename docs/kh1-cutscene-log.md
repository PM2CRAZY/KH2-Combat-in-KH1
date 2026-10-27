# Cutscene log (t706)

The first step toward the QoL cutscene skipper (the user: "add an auto cutscene skipper in qol", then "make sure the log is toggled in the debug menu so we don't bloat performance"). t706 changes no behaviour: it only writes what KH1 does for a cutscene to `kh1-kh2.log`.

**Switch.** The debug menu's Logs page, Cutscenes & Events (ini `log_cutscenes`, off by default). The hooks go in only with `debug_menu = on` at start and only on the Steam build; with the category off each hook costs one atomic load and a call through to KH1's own function. The log stops after 40,000 lines in a session.

**What it logs** (`win/cutscenelog.rs`):

- Scene prepare (Steam `0x14017CB10 (kind, sub, flag)`): the arguments before and after, KH1's event flag (`0x14298C9A0`) and its scene triple (`0x14233FE84 / 8C / 90`). Checked (decomp).
- Event script runner (`0x1401A38A0 (context, index)`): a line when a call moves a script's counter on or a new script starts: the context and code as offsets from the exe, the counter and opcode (the instruction's byte 3) at the start and end of the call, `(end)` on opcode 5, and the cursor. Checked (decomp) for the layout (code pointer `+0x80`, counter `+0x90`, cursor `+0x190`).
- Movie opener (`0x14011C640 (reader, wide path)`): the file name. Assumption: the second argument is the path and this is the only movie path in KH1; the log shows whether so.

**Hook checks.** Each hook compares its function's first bytes (5 whole, position-independent instructions) and refuses on a mismatch. Checked (exe) for the bytes; not run in game.

**Next.** From a log of a cutscene and a movie: where a cutscene starts and ends in the script, and whether it can be skipped by running the script to its end in KH1's own VM (version after this one, only on the user's go-ahead), then movies.
