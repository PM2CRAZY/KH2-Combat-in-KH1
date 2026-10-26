# Project instructions (condensed)

Short version for pasting into the project's instructions. The full rules live in `docs/design-philosophy-instructions.md`, `docs/engineering-rules.md`, `docs/project-instructions.md` and `docs/tester-requests.md`. If this summary and those files differ, the files win.

```
PROJECT
Rust mod (DLL, crates/kh1-hook) that brings Kingdom Hearts 2's combat into Kingdom Hearts 1 (Steam build), with a Config-menu option to switch between KH1 and KH2 behavior per setting. Every setting on KH2 = 1:1 with KH2. Every setting on KH1 = vanilla KH1. You are the lead on the testing branch: all code and docs go on `testing` only, never main or any other branch.

NATIVE ALWAYS
- KH2's numbers and logic run through KH1's own systems. Never offer hybrid or KH2-controller versions, and never ask which version I want.
- Native means extending KH1's engine (new states, table rows and requests, motion slots, particle programs, sound banks, camera modes, functions patched into KH1's own) so KH1 itself steps, draws, plays, collides and ends it. The effects work is the model.
- Not native: a KH2 system running beside KH1's in our own object, interpreter, player or renderer and copying results into KH1's fields. That is "running along". Say so in the thread and ask before shipping anything that way.
- If KH1 lacks something KH2 has, build it into KH1's engine without asking. Ask only if a real attempt can't match KH2 1:1, or when choosing between native designs that change how the feature plays.
- KH1 stays whole: nothing KH1 can do may break, including features KH2 dropped. Where a design choice changes the feel, make it a Config toggle.

1:1 AND SOURCES
- 1:1 = my trace matches KH2 frame for frame, or the KH2 decomp confirms the numbers. Say which you checked and what you couldn't.
- KH2 logic: OG Rust rewrite (Epic addresses, never reuse them), then the KH2 Steam decomp. KH1 internals: KH1 Steam decomp. Confirm KH1's tick rate in the decomp before converting frame counts.
- My trace CSVs and logs are ground truth. Mark every assumption as an assumption and check it.
- Scope is open-ended: report related systems, states and flags you find, and ask before adding them.

PERFORMANCE
As little of our code as possible runs every frame. Prefer data KH1's own code reads over per-frame hooks. Measure before proposing a fix. Game assets are built once from the player's own extracted files into the mod folder, not rebuilt in memory each start.

HARD RULES
- Steam addresses only. New hooks check the function's exact first bytes and refuse on a mismatch.
- No decompiled code or game files in the repo or docs: plain-language notes with a tag (Checked exe / decomp / files / trace, or Assumption).
- Replaced systems must still satisfy what KH1's events and scripts read from them.

VERSIONS
Any change under crates/ bumps VERSION (lib.rs) and adds a matching AGENTS.md stage. One system per version, no unrelated edits, docs-only changes don't bump. After a fix, state the version name (e.g. t306), the 1:1 check you made and what you couldn't check. Don't say whether it was compiled. Run cargo test before pushing.

TESTERS (docs/tester-requests.md)
- Only act on requests written in the tester files. A bug mentioned in chat is not a request.
- When I say "update": pull testing, run tools/testers/waiting.py, move any new thread from Requests into testers/<thread>/<thread>.md word for word, read every waiting thread in full, and reply to each. With several waiting, one sub-agent per thread, each in its own worktree, merged one version at a time.
- Reply in your own `> [!tip] Assistant` box, plain chat. Short questions with options (a), (b), (c), only what changes what you'd build. Never write in a tester's box or add an empty one, and never edit what a tester wrote.
- No code until the tester writes "Go ahead" after your latest message. Their go-ahead counts.
- After replying, run tools/testers/threads.py to rebuild Contents and block ids. Push each thread's reply as soon as it's written, then re-check for threads left waiting before telling me everything is answered.
- "Close": move the thread word for word to tester-requests-archive.md with a Closed line, remove it from Contents, delete its folder.
- Tell me after each round which threads got replies, which versions, what to test, and what's still open.

DON'T
Touch anything outside testing. Write code without a go-ahead. Add requests nobody wrote in the files. Present a guess as fact.
```
