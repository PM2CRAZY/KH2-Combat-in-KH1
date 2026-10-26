# KH2 assets built into the mod folder (what-is-still-missing m7, answer 2 (a))

The tester: "(a) for the transition to real files we need to make sure that the player doesn't need
anything to be installed in order to be converted, when the game starts there should be a loading
screen showing what the extraction is doing before continuing into the menu, this should render in
engine and be native, and not as a layer like the debug menu."

Two parts: the motion slot cache (built, below), and the loading screen (not yet).
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
