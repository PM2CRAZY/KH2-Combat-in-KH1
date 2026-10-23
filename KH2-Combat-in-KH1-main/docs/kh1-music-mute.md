# KH1 music volume, and muting it fully (`music_mute`)

The user asked: "Can you also make it where music can be fully muted in the settings?"
These notes come from the Epic executable (image base `0x140000000`), with the Steam
build checked against them. They contain no decompiled code.

## How KH1 sets the music volume

- **The steps [high].** `0x14012AC50` returns the PC port's settings block, `0x1422DB014`
  (inside the `SET-` block at `0x1422DB000`, 0x1F8 bytes; this is separate from the save
  slots). The four volume steps are i16 values: master `+0x10`, music `+0x12`, effects `+0x14`,
  voice `+0x16`. Steam keeps each one 4 bytes later. The defaults are 10 for all four
  (`0x14012ACC0`).
- **The menu holds them at 1..10 [high].** The settings menu object keeps a working copy at
  `+0xBC..+0xC2`, the values on opening at `+0xC4..+0xCA`, and the maximum (10) at `+0xCC`
  (constructor `0x14014FD30`). The step function `0x140150C60 (menu, row, delta)` clamps
  each value to at least 1 and at most `+0xCC`, then previews it (`0x140150730`). Row 1, the
  music, goes through `0x14014BDA0` to `0x1401169B0 (step)`.
- **Step to volume [high].** `0x1401169B0` only acts on steps 1..10. It sets sound category 1
  to `table[step]`. The table at `0x1403D9B90` is f32 `0.0, 0.1, ... 1.0` and is shared with
  master (`0x140116BB0`), effects (`0x140116EF0`) and voice (`0x1401170A0`). Entry 0 (silence)
  can't be reached, so **the lowest music step is 0.1, which is quiet but still heard.** The
  category volume call itself is `0x14004EA20 (handle, category)` then `0x140054860 (handle,
  volume, fade)` then `0x140054570` (release). `0x140117A60 (category, volume)` wraps those
  three with no fade, and the game itself never calls it.
- **The settings applied at start [high].** `0x14014BBE5..0x14014BC18` calls master, music,
  effects and voice, each with its step.
- **The other writer [high].** `0x1400DDD00 (level, fade)`. For level > 0 it sets categories
  1, 2 and 3 to step / 10 (music `+0x12`, then `+0x14`, then `+0x16`) and categories 5 and 6 to
  the level. For level 0 it sets every category to 0. Its callers are `0x14016E0DD`,
  `0x14016E27A`, `0x14016E36D` and `0x14016EF44` (all with level 1.0 and no fade),
  `0x1401A0109` (1.0, fade 1000) and `0x14017D84B` (0.0).
- **Nothing else sets category 1 [med].** Of the 43 calls to `0x14004EA20`, only
  `0x1401169B0` and `0x1400DDD00` pass category 1. The ones that pass a variable id take a
  sound's own handle and set its own volume, pitch or fade. So a BGM starting or switching
  (rooms, battle and boss music, event scenes) plays inside category 1 and leaves the
  category's volume alone.
- **Categories [high for 1, med for the rest].** 1 is music. 2 and 3 are effects and voice,
  and they swap on the flag `0x1422CA8B4`. The effects step also sets 4 to 8.

## What the mod does (`crates/kh1-hook/src/win/music.rs`)

The ini key is `music_mute`:

- `lowest` (the default): the game's lowest Music volume step becomes silence. Steps 2 to 10
  are unchanged.
- `on`: no music at any step.
- `off`: KH1's own steps, with no hooks installed.

Two hooks, both with position-independent prologues and both mapped for Steam:

- `0x1401169B0` (`push rbx; sub rsp, 0x50`; Steam `0x116870`). When the step is to be silent,
  it calls `0x140117A60 (1, 0.0)` in place of the original. Otherwise the original runs. This
  covers the slider as it moves in the menu, the menu's cancel (which re-applies the old
  value) and the settings applied at start.
- `0x1400DDD00` (`mov rax, rsp; push r14; sub rsp, 0x80`; Steam `0xDDD10`). For level > 0 with
  the music to be silent, the stored music step reads 0 for the length of the call, so the
  game sets category 1 to 0 / 10 = 0 with its own fade. The step is put back right after.
  Effects and voice are computed from their own steps as before.

The log reports `music: muted (...)` and `music: heard again (...)` once per change, and the
`music_mute` mode at start-up.

## Where the user turns it off

Open the game's own settings and set the Music volume to its lowest step. Nothing else is
needed. Raising it one step brings the music back at 0.2. Or set `music_mute = on` in
`kh1-kh2.ini` to have no music at any step.

## Check in game [low until tested]

- Music at its lowest step: silent on the title screen, in the field, on a room change, in
  battle music, in a boss fight, in event scenes, and after a Continue from game over (the
  faded `0x1401A0109` path).
- Effects, voices and KH2's sounds keep their volume at every Music step.
- Moving the slider 1 → 2 → 1 inside the menu: heard, then silent. Cancelling the menu
  restores the previous state.
- The pre-rendered movies (opening, ending) may carry their own audio outside category 1.
  If so, their music follows neither KH1's slider nor this change.
