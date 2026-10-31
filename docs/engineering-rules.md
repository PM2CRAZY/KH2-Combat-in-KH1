# Engineering rules

Detail behind the HARD RULES in the core instructions (`design-philosophy-instructions.md`).

## Steam vs Epic addresses
I play the Steam version. The Rust rewrite and the main branch's addresses came from the Epic exe. The logic is the same between builds; only memory addresses change. Never reuse an Epic address. Find the Steam address by matching the logic in the Steam decomp, and check the bytes before hooking. A hook should refuse to install if the bytes don't match (the main branch's `hook.rs` already does this, so keep it).

The Epic build comes later: for now the testing branch's new hooks are Steam-only (the KH2 sounds
through KH1's engine, for example, play on Steam only). Once the Steam work is done, every
Steam-only hook gets its Epic address too, so the Epic build works exactly like the Steam one
(tester's call, kh2-sounds thread, 2026-10-25).

## No decompiled code in the repo
Don't copy decompiled code or game files into the repo or the docs. Decomp findings go in as plain-language notes (address, what it does, how sure we are), never as pasted decompiler output. Ghidra databases and output stay outside the repo.

## Don't break KH1's story and events
Every system we replace must still do what KH1's events and scripts expect from it. Example: the Dive to the Heart attack tutorial waits on an attack count, and KH2's moves not updating it caused a softlock in the main branch. Before replacing a system, check what in KH1 reads it (events, tallies, scripts) and say what you found.

## Performance
Goal: reduce overhead. In the main branch, KH2's systems (its controller, animation, effects and scripts) run as a second simulation alongside KH1's every frame, and this adds to the lag on low-end machines. Prefer reusing KH1's own systems over running a parallel KH2 system next to them.
- Say whether a design adds hooks, per-frame work or duplicate state on top of KH1, and what that costs on low-end hardware.
- Avoid per-frame allocations, redundant hooks and heavy tracing in normal play.
- Flag anything costly on frame time.
- Don't trade native integration for a small performance win without asking me first.
- Measure, don't guess. Use the profiler (or add one) to find where the time goes, and say which section is costly before proposing a fix.

## Keep the docs current
When we verify something in the decomp, record it in a doc in this folder: the address, what it does, and a Checked tag (Checked in the exe, Checked in a trace, or Assumption). Findings that only live in the chat get lost between sessions.

## Shared findings go to kh-blender too
Engine and format facts about KH1 or KH2 that we find here (file formats, structs, Steam addresses, script calls, collision types, game behavior confirmed by decomp or trace) also go into the kh-blender repo, under `docs/research/kh1/` or `docs/research/kh2/`, with the same tag (Checked exe / decomp / files / trace, or Assumption) and the version they came from (for example "KH2-Combat-in-KH1 t886"). Project-specific decisions, versions and tester threads stay here only. If the same note exists in both repos, keep them in step; the newer one wins. kh-blender's own rules are in its `AGENTS.md` and `docs/research/README.md` (owner's call, 2026-10-31).

## Versioning
Any source change bumps the version (VERSION in lib.rs plus a matching AGENTS.md stage). TESTBRANCH uses the letter T (t300 onward). One system per version, and no edits to unrelated code, so a bug traces back to the change that caused it. After a fix, state only the update name (e.g. "t306").
