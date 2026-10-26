# KH2 assets built into the mod folder (what-is-still-missing m7, answer 2 (a))

The tester: "(a) for the transition to real files we need to make sure that the player doesn't need
anything to be installed in order to be converted, when the game starts there should be a loading
screen showing what the extraction is doing before continuing into the menu, this should render in
engine and be native, and not as a layer like the debug menu."

Parts: the motion slot cache (built), the loading screen (built), package reads once (§3), the
KH2 sound cache (built, §4) and the effect package cache (built, §5).
Addresses are the Steam build's. Tags: **Checked (decomp)**, **Checked (exe)**, **Checked (code)**,
**Assumption**.

## 1. The motion slot cache (built)

What it saves: the KH1-format motion blocks `kh2slots::prepare` bakes at every start (each KH2
motion baked into a KH1 motion by `kh1_data::bake` on KH1 Sora's skeleton). The bake is the costly
part of the start (about 10 ms a motion in the testers' logs, around a hundred motions); the rest of
`prepare` (which slot, which request, KH1's events, the tables) stays as it is and runs every start,
because it is cheap and reads the current settings.

Where: `kh1-kh2-cache\kh2-motion-slots.bin` in the folder the DLL was loaded from (Panacea loads it
from the mod's `kh1\dll\` folder: `win::dll_dir`, `GetModuleHandleExW` on our own code; the game's
folder if that fails). Nothing to install: plain Rust, the player's own KH1 and KH2 files.
**Assumption**: OpenKH's Mods Manager may clear that folder on a Build; then the cache is just baked
again on the next start.

When it is used: the file starts with a manifest of everything the bake reads (`slotcache::Manifest`,
readable text):
- the bake format (`slotcache::BAKE_FORMAT`) and the mod's version (`VERSION`): every new build
  bakes once, so a change to the bake code can never use an old block;
- the game build (Steam / Epic);
- KH1's `xa_ex_0010.mset` (Sora's motion table: the templates, and the idles the retarget rests on)
  and `xa_ex_0010.mdls` (his skeleton), each by a 128-bit hash of its contents and its size;
- every KH2 file read to load KH2 Sora (`kh2_data::Install::load_sora_noting`: `00battle.bin`,
  `03system.bin`, `obj/P_EX100.mset` / `.mdlx`, the Keyblade's `.mset` / `.mdlx`, `00objentry.bin`),
  the same way, and whether they came from the Mods Manager's backup or the live files;
- the ini keys the bake reads: `kh1_slots`, `kh1_leg_ik`, `keyblade_turn`.

If it all matches, each block is taken from the file; else the file is ignored and rebuilt. Each
block also has its own key (`kh2slots::slot_key`: the KH1 template motion's bytes, the KH2 motion,
its field variant, its length, the KH1 events written into it, its travel), so a block is only used
for exactly the bake it came from, and a block for a motion this start doesn't build is dropped.

Robustness: the file ends with a hash of everything before it (a cut-short or damaged file is
ignored and rebuilt); it is written to a temporary file and renamed over the old one (never half
written under its real name); any failure to read or write only means baking as before. A bake
that fails isn't cached. Unit tests: `crates/kh1-hook/src/slotcache.rs` (round trip, a changed
input, a damaged file, dropped blocks, a failed bake, the atomic write).

Log lines: "slot cache: <path> (matches this start's files and settings | none yet | rebuilt: <what
changed> | unusable (<why>), rebuilt; checked in N ms)", "slot cache: N block(s) from the cache, M
baked", "slot cache: written (<path>)" or "not written (<error>); baked again next start".

Cost: one file read and the hashes at start (a few MB of KH2 files hashed: tens of ms), nothing per
frame. The file holds the baked blocks (about 100-200 KB a motion in the logs: some MB in all).
**Checked (code)**; not checked in game.

Memory (the tester, ram-use m1 answer 2 (a): no duplicates). The baked blocks are held once:
- loaded from the file, every block is a part of the file's one buffer (`slotcache::Block`), not a
  copy of its own; baked this start, the bake's own buffer. The cache's write-back list and
  `kh2slots`' build share those buffers (before: the file, the blocks copied out of it, the list's
  clone of each and the build's each, up to four copies while loading);
- the file is written streamed from those blocks (`Cache::write`), not assembled in one more buffer;
- `kh2slots::place` (once, on the game thread, when Sora's motion set is first bound) copies each
  block where KH1 plays it from (16-aligned, never freed: KH1 holds handles to it) and frees the
  build's copy as it goes. Nothing reads the build's blocks after that (`place` is their only
  reader; `PLACED` is never reset, so it never places again, room changes included). **Checked
  (code)**. In the testers' logs the blocks are 16.3 MB for 96 motions, so about 16 MB less after
  placing, and about 16-48 MB less at the start-up peak. **Assumption**: not measured in game.

Repeat bakes (ram-use m1 item 4.7):
- Of the 91 bakes in a tester's log (quick-run-native, 18.4 MB), 7 are the very same bake as
  another (same KH2 motion, KH1 template, length and events: Finishing Plus's next finishers 154,
  155, 166, 183, 184, 194, 197 on a second request of ours), 1.9 MB. **Checked (log)**. With t655
  the cache handed out the file's copy to the first ask only, so the second was baked again at
  every start and the file written again every start. Now an ask for a key already made this
  start gets the same block (`Cache::shared`, logged), nothing baked, one copy in the file and in
  memory while loading. **Checked (code)**, unit test `same_bake_twice_is_one_block`.
- The other repeats (Fire's cast on two templates, swing 151 on several requests, ...) differ in
  template, length or KH1 events, so their bytes differ: not shareable as they are.
- Where KH1 plays them from, each slot still gets its own copy (`place`), the identical ones too.
  Sharing one placed motion between two slots isn't done: KH1's pose pass (`0x140391C70`) writes
  the sampled helper channels into the motion's own helper records (`+0x30`) every pass, so a
  motion is also KH1's scratch, and two slots on one copy (a blend between them) could read each
  other's values. **Checked (decomp)**. KH1's own Sora file never shares one motion between
  slots (118 slots, 118 distinct offsets in `xa_ex_0010.mset`). **Checked (files)**. Sharing them
  would save about 1.9 MB more; left out unless the pass is shown to sample before every use.
  **Assumption**: that it would conflict (not seen happen).

Still done every start, not cached (cheap, or used elsewhere): loading KH2 Sora (also used by the
combo rows, effects and sounds), the baker's retarget set-up, the tables `prepare` makes.

## 2. The loading screen (built)

What the tester asked: at game start, before the title menu, KH1 shows a screen saying what the
building is doing, drawn by KH1 itself (not our overlay renderer like the debug menu), and only then
goes on to the title. The tester's pick (m8, answer 2 (a)): shown only when something is being
rebuilt. Code: `crates/kh1-hook/src/win/loadscreen.rs`. Steam only.

### 2.1 Where KH1 is held (Checked (decomp), Checked (exe))

- KH1's frame (`0x140170D50`, the field mode's per-frame call) runs the game mode's state machine
  `0x140170870` first (its only call site: `0x140170E1A`, bytes `E8 51 FA FF FF`, Checked (exe)),
  then the quit check, the pause machine, the task list, the 2D work `0x14017B190` (call site
  `0x140170FF0`, bytes `E8 9B A1 00 00`, its argument `mov ecx, [0x14233FBE8]` just before it,
  Checked (exe)), the frame end, the pacing and the present.
- The state machine's state is the i32 at `0x14233FBC0` (the machine's own `movsxd rax, [rip+..]`
  at `0x140170874` reads it, Checked (exe)):
  - 0: clears a counter, goes to 1;
  - 1: counts down, then `0x140170630` sets up the systems (sound, pad, the text system
    `0x1402CFA20`, menus, the task list `0x14028A890`, ...) and sets state 2;
  - 2: loads the title program (`host0:/khhd/kingdom1/dktitle.rel`, `0x1403869F0`) and goes to 3;
  - 3 / 5 / 7 / 9: the title / the game run; the rest are mode changes. States 0xF (a movie's end)
    and 0x10 come back to 2 later (the title again); the hold is only ever the first one.
- Skipping the machine's call in state 2 is safe (Checked (decomp)):
  - nothing else in KH1 reads state 2 or waits for it to pass (every other reader of
    `0x14233FBC0` tests 3, 5, 7 or 9);
  - the field frame's other work doesn't depend on it: the window messages and the quit check
    (`0x1422EC090`, which can still close the game while held), the pad, the sound step, the task
    list, the 2D work (its `state > 1` part runs from state 2 already), the pacing and the present
    all run as in KH1's own one state-2 frame;
  - the movie states 0xD..0xF are entered from the title, after state 3: no movie runs before;
  - the systems the title needs are all set up by state 1 before state 2 is set.
- **Assumption**: the window keeps presenting KH1's cleared (black) frame while held (every part of
  the frame that presents runs; not watched in game yet).
- The state machine itself can't be hooked at its start: its first instruction is 4 bytes
  (`48 83 EC 28`) and the next is RIP-relative; the call site redirect avoids that.

### 2.2 How the lines are drawn (Checked (decomp))

- `0x1402CFBF0 (align, colour, x, y, text, scale)` is KH1's text routine; `0x1402D02F0 (colour, x,
  y, text, scale)` is its left-aligned thunk (align 0). (The plan had the arguments in another
  order; this is the decomp's.) Align 1 is right-aligned and 2 centred on x: the routine measures
  the line itself with `0x1402D0E10 (text, scale)`. It builds each glyph's packet on KH1's scratch
  stack (`0x14028DCF0` / `0x14028DD40`, push and pop) and sends it at once (`0x140304BD0`), with
  KH1's font (glyph table `0x142E16BE0`). Codes below 0x15 are controls (1 space, 2 new line, 4 / 5
  / 6 the alignment, 8 a colour, 0x0C the scale, ...); others are glyphs.
- The font is ready in state 2: `0x1402CFA20` (state 1) runs `0x1402D1570`, which reads the font
  table (`exchange/US_font_data_tbl.bin` / `FM_font_data_tbl.bin`) with KH1's blocking read
  `0x14028BB60` and sets the glyph table pointer (Checked (decomp)). **Assumption**: its font
  texture is resident by then too (set up in the same call, `0x1402E9800`; not traced to the GPU).
- KH1's own loading line: the 2D work draws sysmsg `0xF` (`0x1402D1510 (0xF)`) with `0x1402D02F0`,
  colour `0x80C0612C`, x 320, y 404 (the bottom middle of KH1's 640 x 448 screen), scale 15, while
  KH1 loads. KH1's own debug lines are drawn at the 2D work's end with the same text system. Ours
  are drawn right after the 2D work returns (the call at `0x140170FF0` redirected), before the frame
  end and the present: the same place in the frame as those debug lines.
- Text in KH1's codes (`menu::encode`, as the "New Ability" boxes).

### 2.3 What it shows, and when

- Held: from the first frame in state 2 until our loader thread (`pose::load_kh2`, started at DLL
  load) ends, however it ends (a guard is dropped on a failure or a panic too), or 90 s at most
  (the rest then builds behind the title, as before). If the DLL comes in after the title started,
  nothing is held.
- Shown: only once a bake actually runs (the motion slot cache didn't have a motion: a first start,
  a changed file, a mod update, a changed ini key). With a matching cache the title waits, on a
  black screen, only for the loader to load KH2 Sora and check the cache (a fraction of a second in
  the testers' logs: "KH2 Sora loaded ... in 0.2-0.7 s"). Not installed KH2, a missing file, a
  failed bake: the loader ends, the title comes.
- Lines, centred, KH1's font: "KH2 Combat: building from your KH1 and KH2 files" (y 196, white,
  scale 16); "Building KH2 move 151 (12 of 96)" or "Saving the built moves" (y 224, KH1's loading
  blue, scale 15); "This happens once, after an update or a change to your files" (y 404, scale 13).
  n of N counts every motion the slot build makes (from the cache or baked); N is counted from
  the same lists before it starts (`kh2slots::prepare`).
- Released: both call sites get KH1's own bytes back (no code of ours in KH1's frame afterwards).
- Log lines: "loading screen: ready ...", "loading screen: KH1's title waits for the mod's
  loader", "loading screen: title held N ms (F frames, shown | nothing shown); released: <why>",
  or "loading screen: not needed (<why>)".

### 2.4 Still to check in game

1. That the held frames present (a black screen, not a frozen or white window) and the lines show
   in KH1's font where expected.
2. The font texture being ready (if not, the lines would be missing or garbled; the hold itself
   doesn't depend on it).
3. Epic: the same call sites need their Epic addresses (Steam only for now, as the other new hooks).

## 3. Package reads at start: key and indexes read once (ram-use m1 answer 2 (a))

Every file the mod reads from either game's packages (`kh1_data::Kh1Install::read` /
`read_remastered`, `kh2_data::Install::read_file` / `read_any` / `read_remastered_any`) used to read
the whole game executable again (KH2's 8.7 MB, KH1's 7.5 MB) and scan it for the package key, and
read each package's `.hed` index again. **Checked (code)**. Now `KeyTable::from_executable_cached`
and `Archive::open_cached` (`kh2_formats::egs`) keep the key per executable and each index per
`.hed` for the rest of the run (a few hundred KB of index entries in all), read again only if the
file's size or time changed. The first thread to ask reads it; threads asking meanwhile wait for
that read instead of each doing their own. Cuts the start-up peak (several loader threads used to
hold an executable each at once) and the start-up time. **Assumption**: the number of package reads
at start (about a hundred) is from the code, not counted in game.

## 4. The KH2 sound cache (built; ram-use m1, "do all three ... go ahead")

What it saves: the cutting `kh2sound` did at every start. Each KH2 sound the mod plays through
KH1's engine is a one-sound bank cut out of one of KH2's sound banks (`kh2_formats::scd_cut`, at
`kh2_volume`, its bus moved to KH1's effects bus), plus Sora's battle voice bank as it is
(`docs/kh1-sound-native.md`). Cutting them read Sora's effect package `obj/P_EX100.a.us` (which
sound each effect element names), the KH2 sound banks the sounds come from (three, about 28 MB
decompressed, held at once), `00battle.bin` (the voices' table) and
`voice/us/battle/tt0_sora.win32.scd`. **Checked (code)**; the 28 MB was measured on the PC
(ram-use thread).

Where: `kh1-kh2-cache\kh2-sounds.bin`, beside `kh2-motion-slots.bin` (the DLL's folder). Code:
`crates/kh1-hook/src/soundcache.rs` (the file, unit tested), `win/kh2sound.rs` (`load`, `cut_all`,
`hand_to_kh1`).

When it is used: the file starts with a manifest (readable text):
- the cache's format (`soundcache::FORMAT`) and the mod's version (`VERSION`: every new build cuts
  once);
- `ini kh2_volume` (it is in the cut banks' records);
- `wants`: a hash of what is asked for (Sora's effect elements, sound ids, the spell packages'
  elements and their sounds, the voices), sorted. It follows `kh2_sounds`, `reflect`, `drive`,
  the forms and spells loaded, so any setting that changes what's loaded changes it;
- `file <name>` for every KH2 file the cutting read, with a stamp of where it is in KH2's
  packages (`kh2_data::Install::stamp_any`): the archive (and whether it is the Mods Manager's
  backup), the file's index entry (offset, stored and real size) and the `.pkg`'s size and
  modification time; "missing" when no archive has it. So a changed or moved file is noticed from
  the indexes alone, without reading or decompressing it. **Assumption**: anything that changes a
  file in a `.pkg` (a patcher, a Steam repair) changes its index entry or the `.pkg`'s size or
  time; if it didn't, the old sounds would be kept until the next mod version.

The files listed are the ones the file itself names, so the check needs no file read: each listed
file's stamp is worked out now and compared, with the request lines. Anything different, or a
damaged file (the end hash, lengths past the file's end): the sounds are cut again and the file
written again (temporary file renamed over it, as the slot cache). A failure to write only means
cutting again next start.

Memory (one copy):
- From the file: read piece by piece (8 KB buffered) straight into each sound's own buffer, no
  whole-file buffer; those buffers are moved into `kh1bank::add` / `set_voice_bank`, as the cut
  ones were. **Checked (code)**.
- Cutting again: one KH2 bank in memory at a time (the sounds are cut bank by bank, each bank
  freed before the next is read; before, all three stayed until the end, about 28 MB); the effect
  package is freed once the elements' sound ids are known; the cut sounds are written to the file
  and then moved to KH1's engine (no copy). **Checked (code)**.
- So a start with a matching cache reads about 9.3 MB (the sounds, measured in the ram-use thread)
  instead of the effect package, three banks (about 28 MB), `00battle.bin` and the voice bank, and
  decompresses and decrypts nothing for the sounds. **Assumption**: the start-time saving (the
  package reads and inflates, likely a few hundred ms on the sound thread) is estimated, not
  measured.

The loading screen (§2): `kh2sound::start` takes a `loadscreen::job()` before its thread starts,
so KH1's title waits for it too. It is let go as soon as the cache is found to match (the title
then waits only for the motion loader, as before); when the sounds are cut again, the screen
shows "Building KH2 sounds (n of N)" (after the motions' line, if both are rebuilt) until the file
is written. If KH2 Sora doesn't load, the job ends with the loader. The 90 s longest hold still
applies.

Log lines (Loading & Cache): "sound cache: <path> (matches this start's files and settings; N
sound(s) read in M ms)", "sound cache: <path> (none yet | unusable (<why>) | changed: <keys>);
cutting the sounds from KH2's banks", "sound cache: written (<path>; N sound(s) cut in T s)" or
"not written (<error>); cut again next start". The KH1 bank lines (KH2 Sounds) are as before.

Still to check in game: the hit and miss starts, the screen's sound line, and that the sounds
play the same from the cache as cut.

## 5. The effect package cache (ram-use m3: "do all three", the effect packages)

What it saves: each KH2 effect package made into a KH1 DPX (`kh2dpx::build`, `docs/kh2-effects.md`
§18): Sora's `p_ex100`, the Keyblade's `w_ex010`, the forms' (`p_ex100_btlf`, `p_ex100_ultf`,
Wisdom's `p_ex100_magf_a_us`) and the spells' (`reflect_N_mag`, `fire_N_mag`, ...). One file per
package, `kh1-kh2-cache\kh2-effects-<label>.bin`, in the same folder as `kh2-motion-slots.bin`
(`pose::slot_cache_dir`). Code: `crates/kh1-hook/src/dpxcache.rs` (the file, pure Rust, unit
tested) and `kh2dpx::cached_or_built` (the game side). **Checked (code)**; not checked in game.

What a file holds: a manifest (as the slot cache's, readable text), the DPX's bytes as built (before
KH1 relocates them), its MD5, the package's own HD images (each once, however many texture
records take it), each texture record's image (one of its own, or KH2's common image of a TBP),
and what playing it needs (each element's layer: number, KH1 anchor, flags, wait, fade, scale,
offset, lookup word; each element's program types; where KH2's own programs are; its lookup
entries; its per-palette DPDs). KH2's common images (`effect/texcommon.dpd`) are not in it: they
are read once at every start as before (KH1 makes them into textures shared by every package).

When it is used: the file's manifest must equal this time's:
- the build format (`dpxcache::BUILD_FORMAT`) and the mod's version (`VERSION`): a new build
  builds once;
- the game build (Steam / Epic) and which KH2 copy is read (Mods Manager backup or live);
- what the build is given (`kh2dpx::Source`): the package, what it is built for (Sora's: the
  `kh2_effects` list), the elements switched on, the element sets (Sora's: the moves `kh2_effects`
  lets play, the Drive and Revert flashes, Quick Run's start; a form's: its motions), every element
  or only the sets', the KH2 flags left out, KH2's 0x4009 point from the owner's model;
- the package's file in KH2's install as stored in its `.pkg` (its PAX and every HD image: one hash
  of the stored bytes, `Install::read_stored_any`, not decoded), and the bytes the mod holds for it
  if it holds them (a form's archive, a spell's file);
- `effect/texcommon.dpd` as stored (its textures are copied into each DPX, and its images decide
  which records are common).

A file that differs, is missing, damaged (a check over every piece at its end, and lengths checked
before any memory is taken) or can't be read is built again and rewritten (to a temporary file,
then renamed over the old one); a file that can't be written only means building again next time.
Content hashes are `dpxcache::fast_hash` (two 64-bit lanes over 8-byte words; change detection, not
security), so checking a 3 MB package costs about the read of its stored bytes.

When it is read: exactly when the package would have been built before (§18.12 unchanged: Sora's
and the Keyblade's once KH2 Sora is loaded, a form's once he could change into it, a spell's at
the level he has; freed when it can't play). A freed package wanted again is read from its file.
Sora's file is rebuilt when `kh2_effects` changes (same file name).

Memory (ram-use m1 answer 2 (a), no duplicates): the DPX is read straight into the pages it is
played from (no buffer in between); the package's own images into one buffer, each image a part
of it (`slotcache::Block::part`), freed as soon as the package is entered in KH1's loaded files
(KH1 holds its own copy then, as before). A build (no file yet) is as before plus the file
written from the same memory. **Checked (code)**.

Time: builds took 0.01 to 0.08 s each in the testers' logs ("DPX built ... in 0.02 s"), so the
cache saves little start time; it saves the decode of the package and HD images and the conversion
(and their start-up peak: the decoded PAX, all of the file's HD images and the built bytes held
together). **Assumption**: a read from the cache is a few ms (a 0.3 to 3.6 MB file and the stored
package for the check); not measured.

Not tied to the loading screen (§2): the effects builder runs on its own thread and never held
KH1's title; its builds are short.

Log lines (Loading & Cache): "effect cache: <path> (matches this start's files and settings; read
in N ms)", "effect cache: <path> (none yet | rebuilt: <what changed> | unusable (<why>), rebuilt;
checked in N ms)", "effect cache: written (<path>, N images)" or "... not written (<error>); built
again next time"; (Effects) "KH2 effects (native): <label>'s DPX from the cache, N bytes, N effects
in N s".

Disk: about the DPXs' sizes plus their own images: some 5 to 15 MB in all, depending on which
forms and spells have been played (one file per package, overwritten, never more than one each).
