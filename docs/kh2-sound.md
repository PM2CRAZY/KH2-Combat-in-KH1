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
