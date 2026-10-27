# Cutscene Skip (t710)

The QoL row Cutscene Skip (Off | Seen | All, ini `cutscene_skip`, default off) skips the story cutscenes that KH1's pause menu already lets you skip, using KH1's own skip.

## What the user asked

An auto cutscene skipper. Never item shop text, NPC talks or anything the player starts. Skip the dialog cutscenes with high-poly models (the pause menu offers Skip Cutscene) with the game's own function. Never the low-poly dialog scenes that need a confirm press for every line, because they sometimes give items (the Donald and Goofy scene gives dodge roll). Seen = scenes already watched; All = every skippable one. A one second wait before the skip (t710); the user then asked for no wait (t711).

## What KH1 does (all Steam)

- A skippable cutscene turns on bit 0 of the second scene flag word. The pause menu offers Skip Cutscene only then. Checked (decomp). In the user's t709 logs this bit was on in all six high-poly scenes and never in the low-poly dialog scene, talks or the shop. Checked (trace).
- Choosing Skip Cutscene sets one flag byte. KH1's own script step reads it with that bit on, clears it and runs KH1's skip routine (runner bit 0, engine bit `0x400`, fade out, the scene's end). Checked (decomp + exe). The logs show exactly that after the menu: runner 2 to 3, engine word + `0x400`, the scene ending about a second later. Checked (trace).
- The routine that turns the bit on has 13 callers, a position-dependent first instruction and the bit is also cleared directly in other places, so it is not hooked. Checked (decomp + exe).

## What the mod does

With the row on, the mod's pad read (already running) looks at that one word. When the bit comes on it notes the time; the same look, if a skip is due, it writes the flag byte (t710 waited a second; t711 does not). KH1 does the rest. If the scene ends first, a byte that was written and not used is cleared so it can't skip the next scene. Off: one atomic load. Before use the skip routine's first bytes and the script step's read and clear of the byte are checked (they must name that byte); on a mismatch the row does nothing.

- **All:** every skippable scene is skipped.
- **Seen:** KH1 keeps no watched-scene flags (Checked (decomp): strings and the skip path searched). The mod keeps its own list in the ini (`cutscene_seen`): the room (three numbers), the event file and the scene's order in the room visit. A scene not on the list plays and is added; later times it is skipped. The list is shared by all saves (Assumption: delete that line for a new game). Assumption: the order counter restarts when a room is entered again, so a retry after a game over matches.

## Not checked in game

That skipping without opening the pause menu looks the same (the menu also sets its own fade, Assumption: minor); that the key tells scenes apart in every room; the order counter on a retry; Test with the Cutscenes log on (`cutscene skip` lines).

## t711: no wait

The user's t710 test: the high-poly cutscene skip worked but waited; it should skip the first moment the cutscene loads. The wait is gone. In the t710 log the scene's script started 15 to 100 frames before the bit came on, and the script step only accepts a skip with the bit on, so the bit is the earliest point (Checked (decomp + trace)).
