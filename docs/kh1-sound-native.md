# KH2 sounds through KH1's sound system

Research from the t480 round, built t509..t515; the old player removed in the "gut the old
system" round (see "Only KH1's engine" at the end). Thread: `testers/kh2-sounds-through-kh1s-sound-system/`.
All addresses are the **Steam** KH1 exe's (Ghidra address = 0x140000000 + RVA) unless marked.
Where an older doc gives an Epic address, the Steam one is given here and the translation said.

## What we have now

- `crates/kh1-hook/src/win/kh2sound.rs` (testing branch) reads KH2's banks from the user's KH2
  install on a thread (`se/se999`, `se000`, the weapon bank `se500`, Sora's battle voice bank
  `voice/us/battle/tt0_sora.win32.scd`), decodes every needed sound to 16-bit PCM itself
  (`kh2_formats::scd`: Microsoft ADPCM, Ogg Vorbis) and plays each one through its own Windows
  `waveOut` device (`play`: `waveOutOpen` per sound, closed in `tick`). **Checked (code)**
- Volume: it reads KH1's sliders itself at play time (`game_volume`: master x effects or voice,
  from KH1's settings block, Steam `0x14012AF20`, `+0x14` / `+0x18` / `+0x1A`). A slider moved
  while a sound plays doesn't change it. **Checked (code)**
- Not covered: KH1's pause menu, Frame Advance's held sound (`frameadv.rs` calls KH1's sound
  pause, which only knows KH1's own sounds), positional panning (KH1 pans its effects by the
  sound's position against the camera), KH1's stop-all on room changes / cutscenes, the game's
  "sound off" paths. **Checked (code)**
- Cost: one `waveOutOpen` and a PCM copy (when the gain isn't 1) per sound on the game thread,
  and a full decoded copy of every loaded sound in our memory. **Checked (code)**

## Key finding: KH1 and KH2 PC use the same sound engine

- Both exes carry Square Enix's "CDev Sd" sound engine (strings `SQEX::CDev::Engine::Sd::...`,
  `d:\music2soundgit\cdev\sd\...`) with the same SEDBSSCF bank reader: KH1's stream-bank
  read step (in `0x140070000` range, the `'SEDB' 'SSCF'` check) and KH2's (in `0x1400A0000`
  range) are line-for-line the same. **Checked (decomp)**
- KH1's exe has the Ogg Vorbis (libVorbis 1.2.3 / "20090709") and XAudio2 ADPCM stream
  transfer classes, the two codecs KH2's banks use. **Checked (decomp, strings)**
- KH2's files are SEDBSSCF version 3 (se999, the voice bank) and 4 (se000), header size 0x30.
  KH1's own file versions haven't been read (no KH1 `.scd` extracted on the PC yet). **Checked
  (files)** for KH2; KH1 side **Assumption** (same engine, so it should accept both).
- So KH1 can load a KH2 `.scd` from memory as it is, no decoding or re-encoding on our side.
  **Assumption** until tried (the reader is the same code; the bank's layout tables are what
  the engine reads).

## How KH1 plays sounds (Steam)

### The engine wrappers

- `0x14004D530 (data, flag) -> bank handle`: loads a bank from a buffer in memory (the Sd
  driver's load, `0x14005E0B0`), under the engine's lock. **Checked (decomp)**
- `0x14004DBF0 (out, bank handle, sound index, 0, 1, 0)`: creates a sound object of one entry
  of a bank (SoundSystem::CreateSound). **Checked (decomp)**
- `0x14004E9B0 (out, bus)` + `0x1400547F0 (bus, volume, fade)`: a bus's volume. KH1's volume
  setter `0x1400DDD10 (scale, fade)` sets buses 1 / 2 / 3 from the settings block (`+0x16`,
  `+0x18`, `+0x1A` divided by 10: music, sound effects, voice) and buses 5 / 6 to the scale.
  **Checked (decomp)** (the slider-to-bus order; the master slider's own path not traced).

### Banks KH1 knows (the bank list)

- Bank entries (class `Axa::BankEntry_15`) live in a linked list at `0x1404D65D8` (next at
  `+8`): `+0x10` bank number, `+0x14` the file's number (`atoi` of the name after "se"; -1 for
  music / title), `+0x18` the engine's bank handle, `+0x20` / `+0x28` the data pointer, `+0x30`
  the file name. Built by the constructor `0x1400DB1F0`. **Checked (decomp)**, layout matches the
  log's bank-list dump (`[se000.win32.scd] +0x10 2 +0x14 0`). **Checked (trace)**
- `0x1400DFE60 (flag, bank number, data)`: registers a bank already in memory (marks any entry
  of that bank number released (0xFFFF), makes a new entry, loads it with `0x14004D530`) into a
  second list at `0x1404D65B8`. Its only caller is the bank-from-archive load `0x1400D2D10`
  (`.vsb` -> `.win32.scd`). **Checked (decomp)**. That the per-frame step `0x1400DC8C0` moves
  entries from `0x1404D65B8` to `0x1404D65D8` once loaded: **Assumption** (it writes both).
- `0x1400D2C70 (1)` -> `0x1400DBAC0`: releases **every** bank in `0x1404D65D8` and empties the
  list. Called from ten places (`0x14016FFA0`, `0x140170230`, `0x140170300`, ...), most likely
  room / world loads. **Checked (decomp)** that it clears all; what calls it when: **Assumption**.
  So banks we register must be registered again after each such call.
- `0x1400DC8C0` (each frame, from the effect queue step) frees entries whose bank number is
  0xFFFF once the engine says the bank is idle. **Checked (decomp)**

### Effects: one file per sound

- KH1 PC keeps nearly every effect as its own file, `seNNNNNN.win32.scd` with one sound (the
  log: `se034072`, `se021201`, `se060131`, bank -1), plus one indexed system bank, `se000` as
  bank 2. **Checked (trace)**
- `0x1400DE460 SslSeCreate (bank, no, id)` (via `0x1400D2FD0`): if `bank` is the indexed bank
  (`0x1404D5EB8`, which holds 2: **Checked (exe)**) it creates sound index `no` of the entry with
  that bank number; otherwise it finds the entry whose file number (`+0x14`) is `no` and creates
  **sound index 0** of it. The record (0x50 bytes, `mov ecx, 0x50`: **Checked (exe)**, made with
  KH1's allocator `0x14034B54C` and constructor `0x1400DB3B0`) gets the engine sound at `+0x10`,
  bank `+0x30`, no `+0x34`, id `+0x38`, state `+0x3C` (1 effect, 2 / 3 voice), and is linked into
  the live list at `0x1404D6618`. **Checked (decomp)**
- `0x1400DECF0 SslSePlay (bank, no, id)` (via `0x1400D3060`, which skips it while
  `0x1423D3F8C` is set unless a third argument is 0): finds the live record and starts it.
  **Checked (decomp)**. Our sound log hooks both (`soundlog.rs`). **Checked (code)**
- The game side: `0x140179AC0 (no, id, position pointer, volume)` queues a positioned effect in
  KH1's effect queue (`0x14232D560`, 0x20-byte slots, 0x28 or 0x80 of them; bank 2 for `no`
  under 1000, else -1). The queue step `0x1401791E0` (from the field loop) creates and plays each
  one with `SslSeCreate` / `SslSePlay`, pans it from its position (`0x140178030`, clamped to
  0..0x3FFF) and handles stops and fades. Fourteen game functions use it (footsteps
  `0x140295B40`, objects, the attack hits `0x1402ABEA0` / `0x1402AC080`, ...).
  **Checked (decomp)**
- Menu sounds: `0x14017A040 (id)` = create + play of bank 2, `no` = id, centred.
  **Checked (decomp)**

### Voices

- `0x1400DC3E0` (via `0x1400D3370`, the voice create from a `.vsb` archive) makes a bank entry
  per clip and a live record in state 2 / 3; `0x1400DC210 (bank, no)` ("IOP Voice Call") starts
  the state-3 record of that bank / clip. KH1's actor voice functions (`0x1402901F0` group,
  `0x140290930` clip, Steam; the Epic `0x28E060` / `0x28E7A0` in older docs, translated through
  `steam_map.in`) sit on top and decide priority (a clip doesn't cut a higher-priority one).
  **Checked (decomp)** for the call chain; the priority rule from `docs/kh1-voice.md`.

### Pause, Frame Advance, stops

- `0x1400DE010 (mode)` (via `0x1400D2F80`): walks the live list `0x1404D6618` and pauses (1),
  resumes (0) or stops (2) every record, effect and voice alike. KH1's pause machine
  (`0x14017F150`) calls it as the pause opens / closes, and our Frame Advance already calls
  `0x1400D2F80` while it holds the game (`frameadv.rs`). **Checked (decomp, code)**
- So anything that is a record in `0x1404D6618` is paused, resumed, stopped on room exit and
  follows the buses with no extra work. Our `waveOut` sounds are none of that.

### Buses of KH2's sounds

- KH2's sound records (the table from `0x50`) carry a byte at `+1`: 2 in nearly every se999 /
  se000 record, 3 in 31 of the voice bank's 34, 8 in 45 of se500's (the Keyblade bank), 1 in
  what looks like empty records. **Checked (files)**. That this byte is the bus the engine sends
  the sound to (2 effects, 3 voice, as KH1's buses): **Assumption**. If so, se500's weapon
  sounds are on bus 8, which KH1's volume setter never touches (they would ignore the effects
  slider, or be silent if KH1's layout has no bus 8): **open**, has to be checked in game or by
  reading the bus layout.

### Something else in KH1's exe

- KH1 also carries a second, KH2-style sound manager (`Axa::AxaSoundManager`, its own
  `SslSeCreate` at `0x140117CF0`, banks found by number at `+0x48` with an index), reached only
  through a function table (`0x142F21A34`), no direct callers. KH2's own `SslSeCreate` /
  `SslSePlay` strings match that one. Whether KH1 ever uses it: **Assumption** (unused; the log
  only shows the `0x1400DE460` path).

## What reads KH1's sound system (don't break)

- Event scripts and the field play their sounds through the effect queue and the voice calls.
  The queue has 0x28 / 0x80 slots: our sounds take slots too, so a burst of KH2 sounds could
  push out a KH1 one in a busy frame. **Checked (decomp)** (the queue returns when it's full).
- `0x1401791E0` checks `0x1400DEEE0 (-8, -1, -1)` (is a bank -8 sound playing) for the music
  ducking around `se020041` (bank -8 in the log). Our banks must not use -8, -1 or 2 as their
  bank number (-1 is fine only if the file number is unique). **Checked (decomp, trace)**
- `0x1400D2C70 (1)` empties the bank list (above): ours have to come back after it.

## Plan (one system per version)

1. **Load KH2's banks into KH1's engine** (no playing yet). After KH1's sound system is up and
   after every `0x1400D2C70 (1)`, register each KH2 bank we use from our buffer with KH1's own
   loader (`0x14004D530` through a bank entry, the way `0x1400DFE60` does), with bank numbers
   no KH1 file uses, and log the engine's handle and the entry. Check: the log shows the banks
   load, nothing of KH1's changes. Cost: the raw `.scd` bytes stay in our memory (no decoded
   copy), one load per room.
2. **Effects through KH1's create / play.** Two ways, to pick in the thread:
   (a) each KH2 effect we need as its own one-sound bank (cut out of the KH2 bank: its record,
   track, stream; a small SCD writer in `kh2-formats`), registered with a unique file number,
   played through KH1's own queue `0x140179AC0 (no, id, Sora's position, volume)`. Fully KH1's
   path, positioned, paused, stopped, volume by bus; costs the SCD writer.
   (b) whole KH2 banks registered once, and for each play a KH1 record made the way
   `SslSeCreate`'s indexed branch makes it (`0x14004DBF0` with the KH2 index, the 0x50-byte
   record linked into `0x1404D6618`), then KH1's `SslSePlay`. No writer, but ~15 lines that
   mirror KH1's create, and no queue (centred unless we also set the position).
3. **Weapon bank and buses.** Check which bus se500's records go to in KH1 and route them to the
   effects bus if needed (patch the byte in our copy).
4. **Voices.** KH2 battle quotes as KH1 voice records (state 3, `0x1400DC210`), so KH1's voice
   priority and the voice slider apply; or as effects on bus 3 if the voice path proves too tied
   to KH1's `.vsb` archives.
5. **Retire the `waveOut` player** (`kh2sound::play` / `tick`, `game_volume`), keep its loader.

## Not found yet

- KH1's own `.scd` file version and layout (no KH1 sound file extracted for comparison).
- The bus layout KH1 creates (which bus numbers exist, and what bus 8 does).
- What exactly calls `0x1400D2C70 (1)` when (room loads assumed).
- How KH2 itself picks positions / priorities for its sounds (KH2's Steam sound calls not mapped
  here; KH2's ids are `bank * 1000 + index`, `docs/kh2-sound.md`).

## Implementation notes (helper round on the go-ahead)

### KH1's own `.scd` files and the version check

- KH1's room effects are the remastered assets of `amusic/se<world>.dat` (world code from the
  table at `0x1404DCCA0`: dh, di, dc, tw, ...; the name is built by `0x140178E80`). Each asset
  is a one-sound bank named `seNNNNNN.win32.scd` (e.g. `amusic/setw.dat` holds `se017113` ..
  `se021126`, 45 KB to 30 MB). **Checked (files)**, read from the user's install.
- They are SEDBSSCF **version 3** (as KH2's se999 / se500 / voice banks; KH2's se000 is 4),
  header 0x30, the bank number at `0x36` = the file number / 1000 (21 for `se021010`).
  KH1's engine accepts versions 1..4 (`0x140078D40` GetScdHeaderSize: 1 -> 0x10, 2..4 -> 0x20;
  `0x140078DA0` GetSoundBasicDescSize: 1 -> 8, 2..3 -> 0x10, 4 -> 0x20; anything else asserts
  "SCD Version"). **Checked (decomp)**. So KH2's banks are inside what KH1's reader takes.
- The bank layout, read from both games' files (**Checked (files)**): `0x30` u16 sounds, u16
  tracks, u16 audio streams, u16 bank number; `0x38` track table, `0x3C` audio table, `0x40`
  layout table (one per sound), `0x44` routing (0 in every file read), `0x48` attribute table,
  `0x4C` end padding; the sound table at `0x50`. A sound record: byte 0 its track count,
  byte 1 its bus, f32 volume at `0x08`, and its last 4 bytes per track a (u16 track, u16 audio)
  pair. KH2's effect banks have no attributes (the table's offset is the first layout's); the
  voice bank has two. KH1's one-sound files: one sound, track, audio, layout, attribute; in that
  file order: tables, layout, sound, track, attribute, audio, each 16-aligned.
- `kh2_formats::scd_cut::cut` makes a one-sound bank of a KH2 sound in that form. Cutting
  sound 0 of three of KH1's own files (`se021010`, `se017113`, `se021073`) gives back the files
  byte for byte. Every playable id of se999 (96), se500 (44), se000 (124) and the voice bank
  (31) cuts and decodes to the same samples as before, except 35 se000 ids (below).
  **Checked (files)**.
- **The 35 se000 ids.** Our `Scd` reader reads an audio index past the track count (152 in
  se000) through the track table running on into the audio table, so it plays audio
  `index - 152`: e.g. Magnet Burst's 169 (audio 156) has played audio 4, Thunder's 126 / 127 /
  128 / 148 (audio 205 / 207 / 206 / 208) audio 53..56, Reflect's 138 (183), 139 (180), 176
  (184) audio 28..32. The cut keeps the record's own pair, which is what the engine reads
  (KH1's and KH2's reader are the same code), so through KH1's engine these ids play KH2's own
  audio. **Checked (files)** for the indices; that the engine plays the pair's audio:
  **Assumption** (it's the record's only audio reference). The full list of affected ids:
  3, 11, 13, 14, 15, 17, 18, 22, 24, 27, 28, 29, 37, 40, 41, 42, 101, 111, 112, 113, 126, 127,
  128, 134, 135, 136, 138, 139, 148, 153, 164, 169, 172, 173, 175, 176, 177, 178, 180, 300..303,
  306..308, 312 (those that play).

### Registering KH2's banks with KH1's own registration (plan step 1, `win/kh1bank.rs`)

- KH1's room sounds are registered by `0x1400E0F00 (name, bank number, data, flags) -> engine
  handle` (Steam), from `0x1400DB910` (game thread) and from the "_add_scd_thread" `0x1400E0A50`
  (started by `0x1400DBA40`): it skips an entry already there (same name, bank number, flags),
  makes the `BankEntry_15` with `0x1400DB1F0` (file number = `atoi` of the name after "se"),
  loads the bank with `0x14004D530` and pushes the entry on the list `0x1404D65D8` (count
  `0x1404D65E8`). **Checked (decomp)**. `0x1400DFE60` (the doc above) is the voice archives'
  registration into the other list `0x1404D65B8`, not the effects'.
- A bank entry's release (`0x1400DB470` / `0x1400DB7A0`, the vtable's) waits for the bank to be
  idle and frees the data with `_aligned_free`. **Checked (decomp)**. So each registration gets
  a copy made with KH1's own `_aligned_malloc` (its import at `0x1403B0778`, `_aligned_free` at
  `0x1403B0780`, api-ms-win-crt-heap; KH1's loads align 16). **Checked (exe)** (import table).
- `0x1400D2C70 (1)` -> `0x1400DBAC0` releases every bank of the list. Its callers with 1 are
  `0x14016FFA0`, `0x140170230`, `0x140170300`, `0x140170D50`: each runs it in the same sequence
  as other unload steps (`0x14017EF70`, `0x140266530`, `0x1401E1490 (0)`, `0x14018C790`, freeing
  `0x14299C228`, the volume reset `0x1400DDD10 (1.0, 0)`). **Checked (decomp)**; that this is the
  map / room unload: **Assumption** (strongly suggested by the sequence). `0x1400D2C70 (2)` is
  something else (`0x1400E38D0`).
- KH1 registers a whole world's effects at once: Traverse Town's `amusic/setw.dat` holds ~695
  one-sound banks. **Checked (files)**. The engine's bank slot count (`0x14004F570` remaining =
  max `0x142F11814` - used `0x142F11810`; the max set by the driver's init `0x14005D6D0` from
  its config) isn't known statically: it is logged at run time.
- The adding thread holds the sound system's semaphore (`[0x1421AAFD8] + 0x18`) from
  `0x1400DBA40`'s `0x1400E43B0` (WaitForSingleObject) to `0x1400E0A50`'s `0x1400E42A0`
  (ReleaseSemaphore); its thread handle is `+8` while it runs. **Checked (decomp)**.
- What we do: each KH2 sound (and battle-voice clip) the mod loads is cut into a one-sound bank
  (`scd_cut`, volume times `kh2_volume`, one bank a sound) named `se9NNNNN.win32.scd` (file
  numbers from 900000; KH1's own are world * 1000 + n, under 100000), bank number `0x4B320000`
  (KH1 looks entries up by bank numbers from its own data: 2, -8, -1, 90..103, 16-bit values).
  From the frame's end, once KH1's list has stood still for 30 frames after a change and the
  semaphore can be taken (so never while KH1's thread adds), the missing ones are registered,
  16 a frame, only while more than 128 bank slots stay free. Every step is logged
  ("KH1 banks: ..."). Nothing plays through them yet.

### KH2's effects through KH1's effect queue (plan step 2, `kh1bank::play`)

- The queue `0x140179AC0 (no, id, position pointer, volume)` takes a free slot (state byte
  `+0x1C` 0) of the table at `[0x14232D560]` (`0x14232D554` slots, 0x28 or 0x80; 0x20 bytes:
  `+0` no, `+8` the position pointer (kept: read every frame), `+0x10` volume offset, `+0x14`
  bank (2 under 1000, else -1), `+0x18` id) and marks it 1; full, it does nothing. **Checked
  (decomp)**.
- The step `0x1401791E0`: a slot in state 1 is created (`0x1400D2FD0` -> `SslSeCreate`), its
  left / right levels come from `0x140178030` (16383 - d^2 / falloff from the camera's two ear
  points `0x14232DCA0` / `0x14232DCB0`, plus the slot's volume offset, clamped 0..0x3FFF; 0x3FFF
  as the offset means full and centred), the record's position is set (`0x1400DCDB0`), and it
  starts (`0x1400D3060` -> `SslSePlay`) unless `0x1401B37B0` says not now; then state 3, which
  is updated every frame and **never freed by the step**: KH1's callers stop their own sounds
  by (no, id) with `0x140179D90` (frees the slot, stops and deletes the record with
  `0x1400DEF50`) or `0x140179A20`. **Checked (decomp, exe)**: the step's only state writes are
  4, 4 and 3 (Steam `0x1401797EC`, `0x1401797FE`, `0x14017998C`).
- KH1's own actor sounds pass `actor + 0x10` (the position) and volume 0 (e.g. the hit sound
  `0x1402ABEA0`: no 0xA8, id = actor `+4` + 1, stopped later by `0x1402ABF80` with the same
  pair). **Checked (decomp)**.
- What we do: a KH2 sound effect (an effect element's, a frame trigger 8's, a spell element's)
  whose bank is in KH1's list is queued with its file number, an id of its own per play
  (from `0x4B320000`: two plays of one sound can overlap, as KH1's do), Sora's position pointer
  and volume 0. Our slot is stopped with KH1's own stop once its length has played in game time
  (KH1's pause and Frame Advance holds not counted) and KH1's "still playing" (`0x1401789F0`)
  says no, or 2 s after its length. If the bank isn't in KH1's list (yet), the queue is full,
  or the build isn't Steam, it plays on the mod's player as before. Ini `kh2_sound_engine =
  kh1 | mod` (default kh1). Battle quotes stay on the mod's player (step 4).
- So pause, Frame Advance (`0x1400D2F80`, which walks the live records), the volume sliders
  (buses, set by `0x1400DDD10`) and the panning by position are KH1's for them, and KH1's
  map tear-down stops them with KH1's own. **Assumption** until tried in game: the engine plays
  the cut banks (they are in KH1's own file form, and KH1's reader accepts their version), and
  a bank whose engine handle comes back 0 stays on the mod's player (logged).
- Unknown: the falloff `0x14232D500` (KH2's sounds at Sora are as loud as KH1's own sounds at
  Sora, which may differ from before), and what `0x1401B37B0` holds back.

### Buses (plan step 3)

- KH2's volume sliders (Steam KH2): `0x140137D50 (step 1..10)` sets bus 1 (music) and
  `0x140138290 (step)` sets bus 2 (bus 3 instead while `0x1408BBD14` is set) and buses 4, 5, 6,
  7 and 8 to the same level from the table `0x1405AB960`; called from `0x140505D00`,
  `0x140506170`, `0x140506180`. **Checked (decomp)**. So in KH2 the Keyblade bank's bus 8 (and
  se000's 4 / 5) follow the sound-effect slider like bus 2.
- KH1's volume setter `0x1400DDD10` sets buses 1 / 2 / 3 from its music / effects / voice
  sliders and 5 / 6 to its scale argument (1.0 but for its fades); it never touches 4, 7 or 8.
  The same KH2-style bus code is in KH1's exe (`0x140116DB0`, buses 2 / 3 and 4..8) but belongs to
  the second sound manager reached only through its function table (unused, above). **Checked
  (decomp)**. (KH1's own `se017113` is on bus 5, so it doesn't follow KH1's effects slider
  either: KH1's own behaviour, left as it is.)
- So each cut bank whose sound is on bus 4..8 is put on bus 2 (its record's byte 1): KH1's
  effects slider then reaches it as KH2's slider reaches it in KH2. Logged per bank
  ("bus 8 -> 2"). Effects on 2 and voices on 3 are unchanged.

### Battle quotes through KH1's voice path (plan step 4)

- KH1's voice channels: 4 records of 0x28 bytes at `0x142D371D0` (`+0` state, `+1` kind, `+2`
  bank number (i8), `+3` clip (i8), `+8` the actor, `+0x18` an in-memory clip for kind 1,
  `+0x20` from the actor data's `+0x6C`). An actor's channel is its data's (`handle +0x6C`)
  byte `+0xBD`. **Checked (decomp)**.
- `0x140290930 (actor, clip, priority)` (KH1's clip request; the group `0x1402901F0` picks a
  clip and calls it): the channel's standing is 0 free, 1 busy with another actor, 2 busy with
  this actor; it asks only if that is below `priority`, through `0x140290080`, which (voices not
  switched off by `0x142867364` bit 0x100000) for a party member (member id at data `+0x4C`
  under 10) takes the bank number from `0x142D37270[member]` (kind 0) and marks the channel
  state 2; then `0x1402A10B0 (actor, 0x23)`. **Checked (decomp)**.
- The channel step `0x140290A50`: state 2 creates the clip (kind 0: `0x1400D2CB0` ->
  `0x1400DBCF0 (bank, clip)`, the voice bank found in the list `0x1404D65B8` by its number,
  CreateSound of that sound index, a record in state 3 on the live list) and starts the
  speaker's mouth motion (`0x140291FF0`, `0x14029EB20`); state 4 starts it (`0x1400D2CD0` ->
  `0x1400DC210` "IOP Voice Call") and frees the channel once `0x1400D34C0` says it's done.
  **Checked (decomp)**.
- KH1's group rows and KH2's VTBL rows are the same idea: a priority and five (clip, chance).
  KH2's pick (Steam KH2 `0x1403CDB60 (character, id, partner, &priority)`, 14-byte rows:
  character, id, priority, ?, five (i8 clip, i8 chance)): a roll 0..99 (its random number x
  0x10DCD a draw, / 0x28F5C29), choices with a chance subtract it and win when the roll drops
  below 0, those without one are a pool picked evenly; -3 is a partner's line. KH1's
  (`0x1402901F0`) is the same rule. **Checked (decomp)**. KH2 then queues the clip on the object
  if nothing is pending / playing or its priority is at least the pending one's (`0x1403CE580`).
  **Checked (decomp)**.
- Sora's VTBL priorities (**Checked (files)**, `00battle.bin`): 0 (voice 36), 1 (most swings and
  moves), 2 (Explosion's 6, 8), 3 (the hurt lines 0, 1, 29, 30), 4 (2).
- What we do: KH2's whole battle voice bank is registered into KH1's voice list with KH1's own
  copying registration `0x1400E0110 (size, bank number, data)` (it copies the data with
  `_aligned_malloc` and loads it; **Checked (decomp)**), under a voice bank number no entry in
  the list has (127 down to 104; the channel keeps it as an i8), again whenever the list
  changes and ours is gone (KH1 frees an entry another bank of the same number replaced,
  `0x1400DC8C0`). A quote picks its clip by KH2's rule above and goes through KH1's own clip
  request `0x140290930 (Sora, clip, priority)` with Sora's member bank number set to ours for
  that call only: KH1's priority rule, channel, mouth motion, voice bus and pause apply. If
  that can't be done it plays on the mod's player. **Assumption** until tried in game: KH1's
  voice create plays a KH2 Vorbis clip by its sound index (the engine and reader are the same).
- Open: KH2's priority 0 can't pass KH1's rule (nothing is below 0); it is asked at 1 (plays
  when Sora's channel is free), which is what KH2's 0 does when nothing is pending. A -3
  (partner's line) choice is left out as before, so Sora says one of his own there.

## Only KH1's engine (the old player removed)

The tester (m4 answer): "I want you to gut the old system and transfer everything". Since then:

- `kh2sound.rs` decodes nothing. The loader reads each KH2 bank, cuts the wanted sounds out
  (`scd_cut::cut`), hands the cut banks to `kh1bank`, and frees the whole banks. The ~56 MB of
  decoded effects and ~2-3 MB of decoded quotes are gone. **Checked (code)**
- The mod's `waveOut` player (`play`, `tick`, its 16-bit copies), its volume reading
  (`game_volume`, KH1's settings block) and the ini's `kh2_sound_engine` are removed. Volume,
  pause, Frame Advance, panning and room stops are KH1's (its buses and live list), as for its
  own sounds. **Checked (code)**
- A sound KH1's engine can't take at that moment (its bank not back in KH1's list yet after a
  room load, the effect queue full, a voice line KH1's priority rule keeps) isn't heard, as KH1
  drops its own sounds then; the first 20 are logged ("KH2 sound not played: ..."). Off Steam
  nothing is registered, so no KH2 sound is heard there. **Checked (code)**
- Our effect slot in KH1's queue is still stopped by us (KH1's step never frees slots): after
  the sound's length, now read from its audio header without decoding (`scd_cut::length_ms`:
  Microsoft ADPCM whole blocks x samples per block, a part block for the rest), and once KH1's
  "still playing" `0x1401789F0` says no. A stream of another codec (no length in its header) is
  stopped once KH1 has said it plays and then that it doesn't, or after 2 s if it never starts,
  30 s at most. **Checked (code)**; every effect sound the mod uses is ADPCM (codec 12),
  **Checked (files)** (se999 / se000 / se500).
- Battle quotes: the clip list keeps the clips whose sound record has audio (the cut's own
  test), instead of the clips that decoded. **Checked (code)**
