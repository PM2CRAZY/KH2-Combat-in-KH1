# Hiding KH1's field HUD with a key (inventory row 26, main s312)

The owner on main: "can you also make it so that pg dn hides the hud?" (Page Down, configurable in the
ini as `hud_toggle_key`; press again to show). Not built on testing-merge yet: these notes say what
KH1 has, what a native version needs, and what the repo doesn't hold. Steam addresses unless marked.

## What KH1 has

- **No whole-HUD switch.** Main's Epic analysis read every HUD task and found none (main
  `docs/kh1-hud-hide.md`, Epic, its tag [high]); on Steam the same is **Assumption** (not re-read).
- **The event bit `0x142867364 & 8`** (Epic `0x142867CD8`): while set, Sora's HP/MP gauge task
  `0x14026DB00` draws no gauge (Checked (decomp), `kh1-drive-forms-native.md` 14.2.1). Set and cleared
  by KH1's event player, so an event ending would show the gauges again. It hides only the party
  gauges: the command menu (`0x14027B870`), the lock-on cursor, the target gauge, munny, pop-ups and
  info messages keep drawing. Whether anything outside the HUD reads bit 3: not checked (Assumption:
  main found only the gauge task among HUD tasks).
- **The slot fade `0x140268E30 (slot, mode)`** (testing already calls it, `driveparty.rs`): hides a
  party slot's gauge (mode 3), shows it (1 / 2). Gauges only (mode meanings: Assumption, main's notes).
- **The HUD's tasks** (Checked (decomp), `kh1-frame-advance.md`): priority 165001 (`0x1402680F0`,
  `0x1402684A0`, `0x14026FE80`, `0x140270F80`, `0x1402719F0`, `0x140272330`, `0x140274400`,
  `0x140274540`, `0x1402747D0`, `0x1402758B0`, `0x140275B00`), 169999 (`0x140293750`, the command
  menu window's open / close) and 170001 (`0x14026DB00`, `0x140270620`, `0x140272F10`, `0x1402739D0`,
  `0x14027B870`). Each mixes its state (timers, fades) with its draw; on Epic every draw goes through
  KH1's immediate draw (`0x1400FBB60 (count)` -> `0x1400FBBA0`; main, [high]); on Steam the same
  path is named in `kh1-render.md` [med].

## A native version, and what it needs

KH1 draws nothing for an empty packet (a count of 0 is its own path for no quads; main, Epic). So:
the key (read by the mod's monitor thread every 50 ms, beside Insert, not in the game frame) flips a
switch; while hidden, the HUD tasks' draws are given a count of 0 and everything else of theirs
(fades, timers, sounds, the command menu answering the pad) runs as KH1's. To keep the cost off the
frame: the draw hook enabled only while hidden (nothing of ours runs while the HUD shows), and the HUD
tasks told apart by their task nodes (the main list's `+0x10` function, as Frame Advance reads it),
swapped once on the press instead of hooking their registration.

Missing from the repo (needed before any hook):
1. The first bytes of Steam `0x1400FBB60` and `0x1400FBBA0` (the hook's prologue check). From this version
   the log's General category writes them once, five seconds in ("HUD hide research: first bytes
   of ..."), so the tester's next `kh1-kh2.log` gives them.
2. The Steam decomp check that every draw of the 17 tasks above goes through `0x1400FBB60` and
   that a count of 0 draws nothing there (main's finding is Epic's).
3. Whether the HUD's setup runs again on a room change and makes new task nodes (then the swap is
   redone on the room's start; Assumption either way).
