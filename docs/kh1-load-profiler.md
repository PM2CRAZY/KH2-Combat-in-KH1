# The load profiler (t783)

## Why

The owner: every start, loading into the save freezes the game for a moment. With the mod off it
doesn't, so it is ours (the owner, checked in game).

His t782 traces (Checked: trace, `trace_..._f108` and `trace_..._f73`, and the log beside them):

- Every frame is ~16 to 17 ms, except one: 486 ms in the first trace, 423 ms in the second.
- That frame is the first one after the room's opening script hands Sora over: the log's f17
  "KH1 takes him: base Sora", then f18 "KH2's controller ready for Sora" and "KH1 has Sora".
- The log doesn't time any of that frame's work, nor the room load before it (the 207 KH2
  motions, 35 MB, placed into KH1's memory; Sora's motion set bound; the retarget and hair set
  up). Reading the code, creating KH2's controller (`takeover::new_driver`) shouldn't take that
  long (the rig is shared, not copied). Assumption, not measured.

So t783 measures before any fix (the engineering rules: measure first).

## What it does

- Loaded only with the debug tools (`debug_menu = on`) and the Loading & Cache log on at the start
  (`log_loading = on`). Otherwise nothing of it runs.
- A thread of ours samples KH1's game thread (the thread KH1's frame end runs on) about once a
  millisecond:
  - from the game's start until 10 s after the first room is entered (the title, the save's load,
    the room's first seconds);
  - and for 10 s after each later room is entered (the room line in the log opens it).
- A sample: the game thread is paused, its registers and the top 16 KB of its stack copied into a
  buffer made beforehand, and it is let go (nothing allocated, no lock taken while it is paused).
  The return addresses in that copy (values in KH1's code or ours that follow a call instruction)
  are its call stack.
- A frame over 50 ms between two of KH1's frame ends gets one log line (Loading & Cache), at most
  30 per window:
  - its length, the samples in it, and the share with our code anywhere on the stack;
  - **ours, innermost**: our functions deepest on the stack, which is where our time went, each
    with what it was running in (ours, KH1's or the system's code: a copy, an allocation, a file
    read, a wait);
  - **ours, entered by**: our outermost function, which is the hook KH1 called us through;
  - **without ours**: KH1's own functions for the samples with none of ours, as Steam addresses
    (0x140000000-based, as the decomp names them).
- Our functions are named from the build's PDB (the one the build leaves in `target\release`),
  through the system's `dbghelp.dll`, and always carry `ours+offset`. A tester's DLL from GitHub
  has no PDB beside it: offsets only, which can be looked up against that build's PDB.
- When a window ends: "load profiler: window over (N samples, M slow frames reported)".

## Costs

- Outside a window: KH1's frame end reads the clock once. Without the debug tools: nothing.
- In a window: pausing the game thread about a thousand times a second. Assumption, not measured:
  a few hundredths of a millisecond each, so the game runs a few percent slower while it lasts.
  The sampler raises Windows' timer resolution to 1 ms while a window is open.
- Memory: about 5 MB of samples, kept for the session once the debug tools are on.

## Not native

This is a debug tool, not a game system: it runs beside KH1 on its own thread and changes nothing
in the game.

## Next

The t783 log line for the slow frame names the function. The fix goes in its own version.

## Findings (t783 logs, the owner's, 2026-10-28)

Names from the build's PDB (`kh1-kh2.pdb` in the game folder, copied by the build bat). Checked (log + PDB).

- **The freeze**: log frame f21, the frame after "KH2's controller ready for Sora": 829 ms (first run), 337 ms
  (second). 97 to 99% in `ingameskip`'s scene reading (entered from `probe::pad`), all of it in Windows:
  `crash::readable` twice per script word. Fixed in t784 (`kh1-ingame-skip.md`).
- **f1** (the room entered): 132 to 186 ms, mostly ours: `kh2slots` binding Sora's motion set
  (`bind_own`, `pose::file_slot_of_motion`, in Windows), `kh2slots::apex`, `bodyhits::frame`. The binding's
  queries and the 35 MB copy at that frame: t785 (`kh1-asset-cache.md` §7). `bodyhits::frame` and the per-frame
  checks: open.
- **f22**, first run only: 144 ms, 91% in one small function of ours called in a loop from the control step
  (the PDB's nearest name, `Character::growth_level`, may be a folded copy). Didn't repeat. Open.
- **KH1's own**: the save's load (917 ms, KH1's code, `0x1400e4380` most); closing the game (`0x1401791e0`).
- Symbol note: the build's PDB has only public names, so a private function shows as the public one before
  it (`ingameskip::set` for the scan). The offset is exact.

## t785 log and t786

- f21 gone (t784), f1 down to 76 ms (t785), 97% ours, 46% placed in `bodyhits::frame` by the PDB's public
  names. Too coarse to fix from: a private function shows as the public one before it.
- t786: the release build keeps line tables (`Cargo.toml`, `debug = "line-tables-only"`), so the PDB names
  every function; each innermost function in a report lists its top source lines. The PDB is loaded when
  sampling starts (it held the sampler for two frames in the t785 log).
- The debug-only cutscene log (`cutscenelog::frame`, Cutscenes & Events on) took 62% of a 550 ms frame during
  the save's load in the same log: debug tools only.

