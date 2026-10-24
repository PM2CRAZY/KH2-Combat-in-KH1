# Core instructions

Short core block. Detail lives in two other docs in this folder: `systems-to-port.md` and `engineering-rules.md`. Read them when a task touches them.

```
PROJECT GOAL
Replicate Kingdom Hearts 2's combat in Kingdom Hearts 1 (all Rust), with a Config-menu option to switch between KH1 and KH2 behavior per setting. With every setting on KH2, the feel must be 1:1 with KH2.

DESIGN PHILOSOPHY (TESTBRANCH rewrite)
KH1's own systems own Sora. We bring KH2's logic, animations, poses, physics, camera and effects into KH1's engine so they run natively.

NATIVE ALWAYS, AND BUILD WHAT KH1 LACKS
Always implement the native version: KH2's behavior running through KH1's own systems. Don't offer hybrid or KH2-controller versions, and don't ask me which version I want. I will always pick native. Use hybrid or KH2 control only if I explicitly ask for it.

If KH2 has a feature, state, mechanic or piece of code that KH1 doesn't have, don't work around it. Find a way to add it to KH1's engine so the feature exists there, runs natively and matches KH2 1:1. Treat the missing piece as part of the task.

If, after a real attempt, the native result still can't match KH2 1:1, stop and ask me before settling. Tell me what differs, why, and what the native options are. Also ask before choosing between different native designs that change how the feature plays.

Running natively is worth doing even when it saves little performance. Two systems side by side create interaction bugs (the main branch's KH1/KH2 animation conflicts are the example).

TARGET VS METHOD
- Target: with a setting on KH2, every value matches KH2's exe exactly (timings, frame data, momentum, speeds, windows, camera). No approximating.
- Method: those values run inside KH1's engine, through KH1's own systems. We bring KH2's numbers and logic into KH1, not KH2's controller.
- KH1 and KH2 should behave the same in tick rate and fps terms. Confirm KH1's tick rate in the KH1 decomp before converting any frame counts. Don't assume they differ.
- "1:1" is done when my in-game trace matches KH2's value frame for frame, or the KH2 decomp confirms the numbers. Say which you checked and what you couldn't.

SCOPE IS OPEN-ENDED
The systems I list (see docs/systems-to-port.md) are examples, not the full list. I don't know every visible or hidden mechanic in KH1 or KH2. Finding what I missed is your job: look in the KH2 source and decomp, and in KH1, for related systems, states, flags and special cases. When you find one, tell me what it is, where, and whether it matters for 1:1 feel, then ask before adding it. "I didn't ask for it" is not a reason to leave it out.

SOURCES OF TRUTH
- "KH2 source code" means the OG Rust rewrite (OG-KH2-Combat-Rust-Rewrite inside the TESTBRANCH folder).
- Anything not in it: the Ghidra C decomps in the users decomp folder (Steam KH1 and KH2). For KH1 internals, use the KH1 decomp or assembly.
- Mark any assumption about how KH1 or KH2 works as an assumption and check it. Never present a guess as fact.
- My Triangle + Select trace CSVs are ground truth for what I'm seeing.
- The main branch is in the KH2-Combat-in-KH1-main subfolder of the TESTBRANCH folder, for reference.

PERFORMANCE
Reduce overhead. In the main branch, KH2's systems run as a second simulation alongside KH1's every frame, which adds lag on low-end machines. Prefer reusing KH1's own systems over a parallel KH2 system. Measure before proposing a fix, and don't trade native integration for a small speed win without asking me.

HARD RULES (details in docs/engineering-rules.md)
- Never reuse Epic addresses (the Rust rewrite is Epic, I play Steam).
- No decompiled code or game files in the repo or docs: plain-language notes only.
- Replaced systems must still satisfy what KH1's events and scripts read from them.

WORKFLOW
- Don't write code until I give the go-ahead. Questions and design discussion don't count as a request to code.
- One system per version. Don't edit unrelated code.
- Any source change bumps the version (VERSION in lib.rs plus a matching AGENTS.md stage). The TESTBRANCH letter is T (t300 onward).
- After a fix, state only the update name, e.g. "t306". Don't say it hasn't been compiled.
- If a request leaves a design choice open, ask me before choosing.
```

## KH1 stays whole (the tester's design note, kh2-camera-fixes thread, 2026-10-24)

Every agent working on this branch follows this together with the core block above:

- **All settings on KH1 = KH1.** With every Config row on KH1, the game must play and feel
  exactly like KH1, even with our new systems running underneath. If it does, we succeeded.
  The only exception is where a native KH2 system meets one of KH1's.
- **Keep everything KH1 can do.** Nothing KH1 does may break or change because of our work.
  Features KH1 has that KH2 lost (the ones dropped when the series moved to KH2) stay in.
- **Add what's missing to KH1's engine.** KH2's features KH1 lacks are built into KH1's engine
  (see "Native always, and build what KH1 lacks" above).
- **Both native, combined where they can be.** KH1's and KH2's systems are both native, are
  combined where possible, and share the same systems (the example: the effects work gave KH1's
  renderer the extra calls KH2's effects need, instead of a second renderer).
- **Toggles for feel.** When a design choice critically affects how the game feels, it becomes
  a toggle the player can change (a Config row), not a silent change for everyone.
