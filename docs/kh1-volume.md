# KH1 volume sliders: the lowest step is silence (t915)

The owner asked for the lowest setting of every audio slider to mute its track. Code: `crates/kh1-hook/src/win/volmute.rs`, ini `volume_mute` (on by default, read at start).

## How KH1 sets its volumes

- KH1 keeps four slider steps in its settings block (Steam `0x1404D8614`): `+0x14` master, `+0x16` bus 1, `+0x18` sound effects, `+0x1A` voices. Each is a step from 1 to 10. Checked (decomp).
- Its four slider setters only take steps 1..10 and turn a step into a volume through a table of floats in `.rdata` (Steam `0x1403D8B60`: 0.0, 0.1, 0.2 .. 1.0, then 0.0). Checked (decomp + exe). So the lowest step the menu allows, 1, was 0.1: a tenth of full volume, not silence.
  - Master (`0x140116A70`): the sound driver's overall volume.
  - Bus 1 (`0x140116870`). Assumption: music (not checked which sounds use bus 1).
  - Sound effects (`0x140116DB0`): bus 2 (bus 3 while a flag at `0x1422C9D54` is set), plus buses 4..8.
  - Voices (`0x140116F60`): bus 3 (bus 2 while that flag is set; times 0.9 while the flag at `0x1422C9D58` is set).
- Two places read the steps without the table, as step / 10.0. Checked (decomp + exe):
  - KH1's fade (`0x1400DDD10`): when a fade-in ends, it puts buses 1..3 back to their sliders.
  - A stream's start (`0x14011CEA0`) takes the master step. Assumption: the movie player's audio.

## What t915 changes

- The table's step 1 becomes 0.0.
- The four step / 10.0 reads become reads of the same table (same length in bytes; checked against a disassembler, and the registers they use are free at those points).
- Result: step 1 is silence everywhere KH1 sets a volume (the menu, room fades, a stream's start), and steps 2..10 are unchanged. It is all data and code KH1 runs itself, with nothing of ours running per frame. Every patched spot's bytes are checked before anything is written (all or nothing), and it runs on Steam only. With `volume_mute = off`, nothing is patched.

## Buses 5 and 6 after a fade (t916)

- KH1's sound-effects setter sets buses 2 (or 3) and 4..8 to the effects step. But KH1's fade sets buses 5 and 6 to the fade's own level, through its two calls of the bus volume set (`0x1400547F0`, at `0x1400DDEA9` and `0x1400DDED3`). So after every room or event fade those two buses came back at full volume, whatever the slider said. Vanilla does this too. Checked (decomp + exe).
- Seen in game (log): with the effects slider at its lowest, Olympus's room sounds (the wind) came back right after a Trinity. Checked (user, in game, t916): with buses 5 and 6 following the slider, the wind stays muted after the Trinity, so it plays on bus 5 or 6.
- With `effects_after_fade = slider` (the default, read at start), those two calls go through the mod's `bus_after_fade`. A fade-in sets buses 5 and 6 to the effects step's volume from KH1's own table instead (silence at step 1 with Volume Mute), over the same fade time. A fade-out is unchanged. This runs only when KH1 fades, never per frame. With `vanilla`, nothing is redirected.

## Left as it was

- A slider already at step 1 when the game boots: if KH1 applies the saved steps before the mod patches the table, that slider stays at 0.1 until it is moved or the next room fade. Assumption: the mod loads first.
- There is no Config menu row yet (the ini only).

Checked in game (user): t916 working (Olympus wind muted after a Trinity at the lowest effects step). The other sliders and steps were not reported on separately.
