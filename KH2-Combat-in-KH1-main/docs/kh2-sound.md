# KH2 sound effects (b54)

How Sora's moves find their sounds in the HD (PC) release, read from the user's own install.
Our reading, Checked against the files unless marked.

- **Which sound a move plays:** motion trigger 1 starts an effect element of the object's
  effect package (`obj/P_EX100.a.us`, BAR entry "p_ex", PAX). Each PAX element has a `sound`
  field: -1 for none, else `bank * 1000 + index`.
  - Explosion (motion 166): elements 44-47 at frame 0 (44: sound 999038), 48 at frame 34
    (999042).
  - Magnet Burst (motion 197): element 51 at frame 10 (sound 169), 52 at frame 60 (999021).
- **Banks:** `se/seNNN.win32.scd` in the `kh2_*` archives (431 of them; se000 17 MB, se999
  7 MB). SQEX "SEDBSSCF":
  - `0x30`: u16 entry count, u16 stream count, u16 layout count; `0x38`: their offset tables.
  - An entry is a small command script (its stream is the stream of the same index; the
    script's `07 00 03 00 <u32>` value is 4.57 x the stream's length in ms for every one in
    se999), or, in banks with more entries than scripts (se000: 162 scripts, 314 entries), a
    stream header itself.
  - Stream: 0x20-byte header (size, channels, rate, codec, loop start/end, extra size, aux
    count), extra data (codec 12: WAVEFORMATEX for Microsoft ADPCM), data.
- **Decoding:** Microsoft ADPCM (`kh2_formats::scd::decode_ms_adpcm`); matches ffmpeg's
  decoder on 999038 to within 74 of a 6367 peak (prediction rounding).
- **Lengths:** 999038 0.57 s, 999042 1.29 s, 169 0.81 s (mono), 999021 0.94 s.
- `kh2-combat sound --game <install> --id N --out x.wav` writes one for listening (local only).
- In KH1: `kh1-hook/src/win/kh2sound.rs` loads the sounds of the chosen motions' elements
  at start (background thread) and plays them through `waveOut` when the element starts.

## Sound ids are id records, not streams (KH1 stage 41)

The user: Explosion and Magnet Burst didn't sound right in KH1. Two things were wrong.

- **Ids:** a bank's header has, at `0x30`, the u16 id count and at `0x36` the bank number,
  and from `0x50` a table of offsets to id records (contiguous, ending at the first entry
  script). A record: byte 0 = 1 plays (0 = nothing), f32 volume at 0x08, the u32 id at 0x0C,
  and its last four bytes two u16: the sound entry (whose stream is the stream of the same
  index) and a track. Ids skip empty ones, so the id is not the stream: 999038 plays entry 35
  at 0.8, 999042 entry 39, 169 (se000) entry 125, 500042 entry 25. Before this, stream 38 (a
  different sound) played for 999038.
- **Missing pieces:** frame trigger 8 on the Keyblade's motion set (`W_EX010.mset`) plays a
  sound of the weapon's bank (`500042` Explosion, `500044` Magnet Burst at frame 18; KH2's
  `0x1403DA0D0` moves ids 500000..500999 to the equipped weapon's bank, se500..se527), and
  frame trigger 14 a battle voice: `00battle`'s VTBL (character 1 = Sora) maps the id to a
  choice of clips (Explosion: voice 6 → 17/18/19/20; Magnet Burst: 7 → 12/9/22/21/11 at
  frame 0, 6 at frame 50) in `voice/us/battle/<world>_sora.win32.scd`, whose streams are Ogg
  Vorbis (codec 6; the Vorbis headers XOR'd with the key at extra byte 2).
- KH1's own swing voice is not played for moves whose KH2 sounds play.

## se000's records (s178)

The user: Reflect's sounds were wrong. se000's id records are 68 bytes, not 20 as in se999 and
se500, and their last four bytes are two u16 that differ: the first is only a running count
of the ids that play, the second the entry. Read by the first (stage 41 to s177), Reflect's
138 / 139 / 176 played nothing and the others played unrelated sounds (57 of se000's 124 ids
unplayable). Read by the second, each spell's sounds sit in one block (Reflect 138..178 →
entries 180..187, Fire 114..119 → 91..98, Blizzard → 2..8, Cure → 35..37) and 12 ids are
unplayable (empty entries 195..213: Thunder's 126..128 among them). In se999 and se500 the two
are equal, so the sounds already checked there are unchanged. Magnet Burst's 169 now plays
entry 156 (2.7 s), not 125.

## Footsteps: KH1's, at KH2's walk and run feet only (after s312)

The user: "sora plays his KH2 walk sounds during any forward movement at all, including dodge
rolling, quick stepping, aerial dodging, gliding, and many other moments when it shouldnt be
playing. just keep KH1 walk steps in".

- **KH1's steps (exe).** `0x1402939B0 (actor, kind)` (Steam `0x295B40`) plays the floor's sound for
  the world and the material under the actor (`actor+0x39E`). Kinds 0..3 run, 4..7 walk (it
  alternates the foot itself, `actor+0x39C`); 8 the stop from a run (`0x1402A7693`,
  `0x1402CA135`); 10 the landing (`0x1402BF26E`, in KH1's touchdown `0x1402BF100`). Vanilla's
  walk and run steps are sound events of KH1's walk / run motions: `0x14029DA4E` →
  `0x140293290`, event id → kind by the table at `0x140529788` (1 / 2 → 4 / 5, 3 / 4 → 0 / 1,
  6 → 10).
- `0x140293C70` (Steam `0x295E00`), hooked since Stage 155 as "the footstep", is not the walk
  steps: its callers are KH1's touch / push routines (`0x1402C0D30`, `0x1402C0ED0`, the crate
  push among them). Left as it was.
- **Under KH2.** KH1's own are dropped while KH2 has Sora (KH1 keeps cycling its walk / run
  whenever the stick is held, whatever KH2 plays). We call `0x1402939B0` ourselves.
- **The bug.** Every KH2 frame trigger 2 played a step, not only the walk / run loops'. KH2's
  motions carry many (`P_EX100.mset`, frame: kind):
  - the dodge roll (209): 26: 10;
  - the landing (5): 2: 10, after every Quick Run, Aerial Dodge or glide that comes down;
  - the jump (3): 2: 9, and the High Jump (201): 1: 11;
  - the combo (151..155), the ability moves (161..173), the hit reactions (11..19) and the
    repels (41, 42): kinds 0 / 4 / 9 / 10;
  - the forms' moves: Valor's and Limit's combos, Anti's above all (its combo, its Anti Glide).
  The run loop's steps also sounded on any motion the rig held in a free state (a dodge roll's
  tail is "Run (motion 209)").
- **Now** (`takeover::walk_steps`): steps only while he is on the ground, in the walk or run state,
  with KH2's walk (1) or run (2) the motion and the rig's track. They sound at that motion's
  own foot contacts (its trigger 2 frames: walk 26 / 62 battle, 16 / 48 field; run 8 / 24
  battle, 12 / 32 field), feet kinds only. No step in any action: dodge roll, Quick Run, Aerial
  Dodge, glide, jumps and landings, attacks, guard, reactions, magic.
- **Forms** follow their own walk / run's contacts. Valor's (B001 / B002), Final's (D001 /
  D002), Limit's (Sora's) and Anti's (F001 / F002) have them. Wisdom's (C001 / C002) and
  Master's (E001 / E002) have none (they skate), so they step nowhere.
- **Not footsteps, unchanged:** KH2's movement sounds of frame trigger 8 (bank 0: the walk's 300,
  the run's 301 / 302, the jump's 303, the dodge roll's 299; Anti's run 503032). The walk and run
  loops never send them (the controller plays loops without frames of its own); the actions'
  play only where `kh2_sounds` or a form allows.
