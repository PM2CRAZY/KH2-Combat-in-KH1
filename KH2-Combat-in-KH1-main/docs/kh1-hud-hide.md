# KH1's field HUD, and hiding it with a key (`hud_toggle_key`)

The user asked: "can you also make it so that pg dn hides the hud?" These notes come from the
Epic executable (image base `0x140000000`), with the Steam build checked against them (every
address below is in `steam_map.in`; the three hooked prologues are byte-identical in both). They
contain no decompiled code. Code: `crates/kh1-hook/src/win/hud.rs`.

## No single HUD switch [high]

- Events set and clear bit 3 of the world flags word `0x142867CD8` (`0x1401A77C0` sets it,
  `0x1401A77B0` clears it, both in a function table at `0x1404FB8D0`, the event opcodes [med]; many event starts set
  bits 2 and 3 together, `or 0xC`). Of the HUD's tasks only `0x14026B990` reads that bit: while
  it is set, that task skips its drawing. No other HUD task reads it.
- The command menu fades itself (`0x140279700`: an alpha at `0x14285320C` stepped by a speed at
  `0x142853210`, clamped to 0..1). The other pieces keep their own state the same way.
- So there is no flag to set; the mod hides the draws instead.

## The HUD module [high for the list, med for each task's role]

`0x140264C50` sets the HUD up (called from `0x14016E5B0` and `0x14020BAB0`). Each of its setup calls first asks whether its task is there yet
(`0x1402824F0 (fn)`, which searches the main task list `0x142867D80` with `0x140288750`) and
registers it if not (`0x140282500 (priority, fn)` -> `0x140288780`). Sixteen tasks:

| Priority | Task | What it draws |
|---|---|---|
| 165001 | `0x140265F80`, `0x140266330`, `0x14026DD10`, `0x14026EE10`, `0x14026F880`, `0x1402701C0`, `0x1402723D0`, `0x140273740`, `0x140273990` | gauges and other HUD pieces (not told apart one by one) |
| 165001 | `0x140272290` | the lock-on cursor (`0x1402729B0`) |
| 165001 | `0x140272660` | the target's gauge (reads the lock state `0x142DB3F10`) |
| 170001 | `0x14026B990` | the HP/MP gauges (Sora's record `0x142812400`, slot n at `+ n × 0xDF0`); reads the event bit above; runs after the command menu (registered later at the same priority) |
| 170001 | `0x14026E4B0`, `0x140270DA0` | HUD text and numbers |
| 170001 | `0x140271860` | the info messages (a queue at `0x14284E360`, pushed by `0x1402712A0`) |
| 170001 | `0x140279700` | the command menu, its lists (`0x140277F40`), the shortcuts |

Nothing else references these functions. Every draw they make, followed through their direct
calls, goes through KH1's immediate draw `0x1400FBB60` (-> `0x1400FBBA0`): sprites (`0x14027A840`,
`0x14027D670`, `0x14027D850`, `0x14027DA50`) and text (`0x140301FB0`, `0x140302430`). They
draw at once; nothing is left for a later task to draw.

The task list: the runner `0x140288590` calls each node's function (`+0x10`) with the node;
bit 2 of the return removes it. While the pause menu is up, the run mask `0x142867D74` is 1,
so only the pause task runs and the HUD tasks don't.

## What the mod does

- **Draws emptied, logic kept.** Each HUD task is registered as a stub of ours (a hook on
  `0x140282500`), which calls KH1's function. While the HUD is hidden, the stub notes its thread
  for the length of the call, and the immediate draw (`0x1400FBBA0`, hooked) is given a count of
  0 on that thread. A count of 0 is KH1's own path for an empty packet (`0x14027A840` with no
  quads), which resets the vertex batch and draws nothing. The tasks' fades, timers, sounds and
  message queue run as before.
- **Lookups stay right.** KH1's find (`0x140288750`, hooked) looks for the stub when asked for
  a HUD function, so the setup, whenever KH1 runs it again, finds its tasks and
  doesn't add a second copy.
- **Never stuck.** Nothing of KH1's is written: the hidden state is the mod's alone, so events
  showing or hiding pieces don't change it, and the next press shows everything again.
- **The key.** The mod's monitor thread looks at the key every 50 ms (as it does for the Insert
  test key). A press counts only while a window of the game has the keyboard, and flips the HUD
  once; holding the key does nothing more. The log says "HUD hidden" / "HUD shown".

## Not hidden

Anything drawn outside these sixteen tasks keeps drawing: the pause menu (task 159999), events
with their subtitles and message windows, the system menus. KH1's and the mod's 3D
effects are drawn by the world and effect passes, not the HUD tasks.

## To check in game

- Field, battle: the gauges, command menu, shortcuts, lock-on cursor and target gauge all go;
  commands, shortcuts and lock-on still work.
- The Drive page and list (D-pad left) go too.
- Pause with the HUD hidden: the pause menu shows; after it, still hidden.
- A room change, an event, game over and the title: the HUD stays as the toggle left it, and
  one press brings it back whole (no piece drawn twice or missing).
