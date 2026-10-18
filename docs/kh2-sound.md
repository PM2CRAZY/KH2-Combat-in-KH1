# KH2 sound effects

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
