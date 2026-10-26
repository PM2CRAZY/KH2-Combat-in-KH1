# KH2 assets built into the mod folder (what-is-still-missing m7, answer 2 (a))

The tester: "(a) for the transition to real files we need to make sure that the player doesn't need
anything to be installed in order to be converted, when the game starts there should be a loading
screen showing what the extraction is doing before continuing into the menu, this should render in
engine and be native, and not as a layer like the debug menu."

Two parts: the motion slot cache (built), and the loading screen (planned below, not built).
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

Still done every start, not cached (cheap, or used elsewhere): loading KH2 Sora (also used by the
combo rows, effects and sounds), the baker's retarget set-up, the tables `prepare` makes.

## 2. The loading screen (plan; not built)

What the tester asked: at game start, before the title menu, KH1 shows a screen saying what the
building is doing, drawn by KH1 itself (not our overlay renderer like the debug menu), and only then
goes on to the title.

### 2.1 Where KH1 can be held (Checked (decomp), Checked (exe))

- KH1's frame (`0x140170D50`, the field mode's per-frame call) runs the game mode's state machine
  `0x140170870` first (its only call site: `0x140170E1A`, `E8 rel32`), then the pause machine, the
  task list, the 2D work `0x14017B190` (call site `0x140170FF0`), the frame end and the present.
- The state machine's state is the i32 at `0x14233FBC0`:
  - 0: sets a counter, goes to 1;
  - 1: counts the counter down, then `0x140170630` initialises the systems (sound, pad, text
    `0x1402CFA20`, menus, the task list `0x14028A890`, ...) and sets state 2;
  - 2: loads the title program (`host0:/khhd/kingdom1/dktitle.rel`, `0x1403869F0`) and goes to 3;
  - 3 / 5 / 7 / 9: the title / the game run; the rest are mode changes (load, new game, movies at
    0xD..0xF).
  The title's own code is in `dktitle.rel`, not the exe, so the exe's last point before the title
  is state 2.
- The plan: redirect the call at `0x140170E1A` (`hook::redirect_call`, exact bytes `E8` + the
  rel32 to `0x140170870`, refusing on a mismatch) to a wrapper that, while state is 2 and our
  building isn't finished, returns without calling the state machine: KH1 keeps running its frames
  (task list, 2D work, present) with the systems `0x140170630` set up, and the title isn't loaded
  yet. When the building is done (or failed: it then falls back to baking in the field as now), the
  wrapper calls the state machine again and state 2 loads the title as usual. KH1's own frame pacing
  and present carry on all along (no frozen window). **Assumption**: nothing else in KH1 needs
  state 2 to pass within a time limit (to check in the decomp: the sound and movie start-up).
- The state machine itself can't be hooked at its start: its first instruction is 4 bytes
  (`48 83 EC 28`) and the next one is RIP-relative (`48 63 05 rel32`, the state), **Checked
  (exe)**; the call site redirect avoids that.

### 2.2 How KH1 draws text itself (Checked (decomp))

- `0x1402D02F0 (colour, x, y, text, scale)` draws a string of KH1's text codes (thunk of
  `0x1402CFBF0 (0, x, y, colour, text, scale)`): it measures with `0x1402D0E10 (text, scale)`,
  builds a 2D packet in KH1's per-frame packet list (`0x14028DCF0 (size, align)`) and draws it with
  KH1's font (glyph table `0x142E16BE0`). Exact first bytes `48 83 EC 38 8B 44 24 60 89 44 24 28 4C
  89 4C 24` (Checked (exe)).
- KH1 itself uses it for its own loading message: the 2D work `0x14017B190` draws sysmsg `0xF`
  (`0x1402D1510 (0xF)`, the system message table at `0x142E1F460` / `0x142E1F468`) with colour
  `0x80C0612C` at x 320, y 404 (the bottom middle of KH1's 640 × 448 screen), scale `0xF`; also
  through `0x14017AD60` (call site `0x14017B323`). **Assumption**: sysmsg `0xF` is KH1's "Now
  loading" line (not read from `sysmsg.bin`).
- Our text would be written in KH1's text codes (`menu.rs`'s plain-string encoder, as the "New
  Ability" boxes are) and drawn with that same call, each frame, from the 2D work's time (a wrapper
  on the call at `0x140170FF0`, after KH1's own 2D work), so it lands in KH1's packet list for that
  frame. **Assumption**: drawing it from the state machine's time (step 1 of the frame) instead
  could be lost to a packet list reset later in the frame; not checked, hence the 2D work's call.

### 2.3 What the screen would show

KH1's black screen (nothing else is drawn before the title), with a few lines in KH1's font, e.g.:
"KH2 Combat: building from your KH1 and KH2 files", then the step ("Loading KH2 Sora", "Checking the
motion cache", "Baking KH2 motion 151 (12 of 96)", "Saving"), from a progress value the loader
thread writes (an atomic step and count; no lock on KH1's thread). With a matching cache the hold
is a frame or two (the loader thread starts at DLL load, before state 2: `win::mod.rs` calls
`pose::load_kh2`), so most starts show nothing.

### 2.4 What's left to check before building it

1. That the frame runs its present and 2D work every frame while state 2 is held (the task list is
   created in `0x140170630`, so it should; to watch in game).
2. Sysmsg `0xF`'s text, and the colour word's layout (KH1's own call is the safe copy).
3. That nothing is timed against the start-up while state 2 is held (the movie states 0xD..0xF are
   entered from elsewhere, not read here: **Assumption** that they come after the title).
4. Epic: the same calls need their Epic addresses (Steam only for now, as the other new hooks).
