# Add-ons: a guide for add-on authors (testing-merge, tm44)

KH2 Combat for KH1 (`kh1-kh2.dll`) can take third-party add-ons: an add-on brings its own row of
abilities to KH1's Abilities menu (after the mod's "KH2" row), learned at a level or a boss and
equipped with Sora's AP like the KH2 ones. An add-on is **data first**: one `addon.toml` the mod
reads at start. A small **C API** answers questions at run time (is my ability equipped? is my
add-on loaded?). Add-on code never runs inside the game's combo or frame: everything an add-on adds
is played by KH1's own systems from the data.

The owner's decisions (chat, 2026-11-07): third-party add-ons yes; each add-on gets its own row;
both a data file and a versioned C API. Rules: data, not code, in the combo; motions only in a
format the mod already loads; DLL-load-order safe; add-ons can't change KH1's abilities or the
KH2 row; with no add-on installed nothing changes; saved abilities keep their meaning when add-ons
are added, removed or reordered.

**What works in tm44:** rows, abilities, learning, the "New Ability" boxes, equipping with AP,
the save, the C API. **What doesn't yet:** finishers are read and checked but not played (section
7). Nothing here is checked in game yet.

## 1. Where an add-on goes

```
<KH1's game folder>\kh1-kh2-addons\<any folder name>\addon.toml
<KH1's game folder>\kh1-kh2-addons\<any folder name>\motions\...anb     (finishers' motions)
```

The mod also looks in a `kh1-kh2-addons` folder beside `kh1-kh2.dll` (Panacea's `dll` folder when
OpenKH's Mods Manager installed the mod), so an OpenKH mod can ship an add-on as assets copied to
`dll/kh1-kh2-addons/<name>/...` (Assumption: the Mods Manager copies a mod's `dll/...` assets with
their folders; not tried). The game folder is read first; within a folder, add-on folders are taken
in name order, and that is the order of their rows. An add-on whose `key` is already loaded is
refused.

Only the files are read (once, at start, by `DllMain`): no timing, no DLL needed. With the debug
tools on (`debug_menu = on` in `kh1-kh2.ini`), `kh1-kh2.log` says what was loaded, refused or
changed (lines starting `add-ons:`).

## 2. `addon.toml`

TOML. A full example (text only, no motion files): `crates/kh1-hook/addons-example/kh3d-example/addon.toml`.

```toml
[addon]
format = 1                 # required: the data format. Format 1 never changes.
key = "kh3d.dreamdrop"     # required, stable forever: 3..64 of a-z 0-9 . _ -
name = "Dream Drop Distance moves"
row = "3D"                 # required: the row's title in the Abilities menu (12 characters)
version = "1.0.0"          # yours, for the log

[[ability]]
key = "blizzard-edge"      # required, unique in the add-on (3..64 of a-z 0-9 . _ -)
id = 1                     # required, 1..127, unique in the add-on: NEVER change or reuse it
name = "Blizzard Edge"     # required, 24 characters
help = "Finisher: end a combo with an icy slash."   # 96 characters
ap = 3                     # required, 0..99
learned = { level = 15 }   # required: "start", { level = N } (1..100) or { boss = "Name" }
icon = "action"            # "action" (as KH1's Slapshot, default) or "support" (as MP Haste)

[ability.finisher]         # optional (section 7: read and checked, not played yet)
motion = "motions/blizzard_edge.anb"     # required: Sora's body, a KH2 ANB (section 3)
keyblade = "motions/blizzard_edge_w.anb" # optional: the Keyblade's own motion
kind = "ground"            # "ground" (default) or "air"
range = { kind = "none" }  # "none" (default), or { kind = "primary" | "any", near = 0, far = 300 }
hits = [[14, 22], [30, 38]]  # required: hit windows, 60 Hz frames from the motion's start, end <= 600
power = 1.25               # 0..10, KH2's attack power factor (default 1)
reaction = "flinch"        # "flinch" (default) or "launch"
effects = [[14, 31]]       # optional [frame, element] of Sora's KH2 effects (section 5)
sounds = [[14, 500042]]    # optional [frame, KH2 sound id] (section 5)
```

- **Errors are logged, never fatal.** A problem in `[addon]` refuses the add-on; one in an ability
  refuses that ability; one in a finisher drops the finisher and keeps the ability. Over-long texts
  are cut, with a note.
- **Text.** KH1's menu font has `A-Z a-z 0-9`, space and `- . , ' ! ? ( ) / : + &`; anything else
  shows as a space (noted in the log).
- **Bosses** for `learned = { boss = "..." }` (case and spaces ignored): Sabor, Cerberus, Captain
  Hook, Clayton, Dragon Maleficent, Pot Centipede, Jafar, Genie Jafar, Chernabog. These are the
  story points the mod already uses (`growth.rs`, the same world and progress values; Assumption
  there: main branch's story bytes). Ask for more and they are added to the mod.
- **When abilities are given.** Once a second while Sora plays (KH1's menu step the KH2 abilities
  use), every ability whose point is reached and isn't in the row yet is put in it, off, with KH1's
  "New Ability" box (one or two at once named, more as "N <row> Abilities"). A save loaded past an
  ability's point gets it at once. Only with the ini's `abilities = kh1` (the default) and on a
  build where the KH2 row works, like the KH2 abilities.

## 3. Motions: the format

A finisher's motion must be in a format the mod already loads for KH1's motion slots: a **KH2 ANB**
(one entry of a KH2 `.mset`). That is a BAR (`"BAR\x01"`) holding an **interpolated motion** (BAR
entry type 9) and, optionally, its **triggers** (type 16), exactly as each `anb` entry (type 17) of
`obj/P_EX100.mset` (KH2 Sora). The mod reads it with the same code that reads KH2 Sora's own
motions (`kh2_formats::motion::Anb`, `kh2_data::sora::clip_from_anb`). OpenKH's tools export and
import these entries.

- **Skeleton:** KH2 Sora's (`P_EX100.mdlx`, the model the mod already loads). The Keyblade's
  motion (`keyblade`) is for KH2's Keyblade bones (`W_EX010`), as KH2 Sora's motion sets pair them.
- **Raw motions** (KH2's other motion kind) aren't read. 3DS formats aren't read.
- **Length:** the motion's frame range at its own rate, counted at 60 Hz (KH1 and KH2 both step
  at 60; the mod's motion slots are 60 Hz). `hits` must fit in it.
- **Root motion:** a motion that moves Sora carries it in its root translation, as KH2's do; the
  mod bakes it into KH1's travel for the slot as for KH2's moves.
- **Triggers** inside the ANB (hit windows naming KH2 attack rows, effect and sound triggers) are
  KH2's own; the TOML's `hits`, `effects` and `sounds` are the add-on's (so an add-on needs no KH2
  attack row numbers).

At start the mod reads each finisher's file and checks it parses as an interpolated KH2 motion
(its length logged); a missing or wrong file drops that finisher (logged).

## 4. Ids and the save

KH1 keeps abilities as 7-bit ids. KH1's own are 1..65, the mod's KH2 ones 66..92; 93..127 (35) are
free, and they are shared by every add-on **at run time only**. In the save each add-on has its own
48-byte list holding **its own ids** (the `id` you wrote), found by its `key`:

- KH1's config block (save sub-block 5, saved with the slot) `+0x78..+0x38F`: an 8-byte header
  ("K2AD", layout 1, 14) and **14 slots** of 56 bytes: the key's hash (FNV-1a 64) and the list
  (KH1's format: an id, bit 7 = not equipped, 0 ends it). KH1 never reads past `+0x78` of that
  block (Checked (decomp, Steam), `docs/kh1-config-menu.md` 1.1); the mod's KH2 list is
  `+0x3C0..+0x3EF`. The area is written only once an add-on is installed, and only when it is all
  zero or already the mod's (anything else there: left alone, no add-on rows on that save).
- At start each loaded ability gets a runtime id from 93 up, in load order. KH1's menu sees runtime
  ids only for the length of one of its own calls; they are turned back into yours as the call
  ends. So **adding, removing or reordering add-ons never changes what a saved byte means.**
- **Removing an add-on:** its slot is left as it is; its row isn't shown. Put it back and its
  abilities are there again. An ability taken out of an add-on's file is kept in its slot (hidden
  from the menu) and comes back if the ability does. 14 slots: when every one belongs to an add-on
  (installed or not), a new add-on gets no row on that save (logged). Slots are never reclaimed.
- **Limits:** 3 add-on rows (`addons::MAX_ROWS`; more installed are refused, logged), 35 abilities
  for all add-ons together (an add-on that doesn't fit whole is refused), 48 per add-on.

## 5. Effects and sounds

- **Effects** are elements of KH2 Sora's effect package (`obj/P_EX100.a.us`, PAX "p_ex"), the ones
  KH2's own moves start with their trigger 1 (`win/kh2fx.rs`: Guard Break 31, 30, 32; Finishing
  Leap 33, 34; Retaliating Slash 39; Counterguard 40, 41; Flash Step 53..56; `docs/kh2-effects.md`).
  The mod's log (Effects, with the debug tools) names each element as it plays.
- **Sounds** are ids of KH2's sound banks the mod reads (`win/kh2sound.rs`): Sora's effect sounds
  (e.g. 999038, 999042 Explosion), the Keyblade's bank (500042 / 500044), as KH2's motion triggers
  name them. The Sound log names each id as it plays.

(Neither plays for an add-on yet: section 7.)

## 6. The C API

For an add-on with code of its own (a DLL). **Version 1 never changes**; anything new comes as
version 2 beside it, and version 1 keeps working. Header: `crates/kh1-hook/include/kh1kh2_addon.h`.

```c
const kh1kh2_addon_api_v1 *kh1kh2_addon_api(uint32_t version);   /* exported by kh1-kh2.dll */

struct kh1kh2_addon_api_v1 {
    uint32_t size, version;                                   /* sizeof, 1 */
    int32_t (*is_equipped)(const char *addon_key, const char *ability_key); /* copies, -1 unknown */
    int32_t (*is_learned)(const char *addon_key, const char *ability_key);  /* 1 / 0, -1 unknown */
    int32_t (*addon_loaded)(const char *addon_key);           /* 1 / 0, -1 bad argument */
    int32_t (*ability_id)(const char *addon_key, const char *ability_key);  /* your id, 0 not loaded */
    int32_t (*on_ready)(kh1kh2_ready_fn cb, void *user);      /* 1 queued */
    void    (*log)(const char *addon_key, const char *msg);
};
```

- **Finding it.** `GetModuleHandleA("kh1-kh2.dll")` (Panacea's dll folder), else
  `GetModuleHandleA("dinput8.dll")` (installed in KH1's folder), then
  `GetProcAddress(m, "kh1kh2_addon_api")`. Check `version == 1` and `size` at least what you use.
- **Timing.** Panacea loads its DLLs in folder order and calling into another DLL from `DllMain` is
  never safe (the loader's lock): **don't call the mod from your `DllMain`**. Call it from Panacea's
  `OnInit` / `OnFrame` exports, a thread of yours, or any later point; if the mod isn't loaded yet,
  try again later. Data files need no timing at all.
- **Ready.** `on_ready(cb, user)` calls `cb(api, user)` once, on KH1's game thread at the end of a
  frame, once the add-ons' files are loaded (in practice the first frame end after your call:
  before a save loads if you call it at start). Never inside one of KH1's moves.
- **Answers.** `is_equipped` / `is_learned` answer from a copy of the rows the mod's menu step makes
  once a second while Sora plays (up to a second behind the menu; -1 at the title, before a save,
  with `abilities` other than `kh1`). Any thread; never blocks on the game.
- **Safety.** Every function takes null or bad UTF-8 and answers -1 / 0; nothing of ours unwinds
  into your code. `log` writes "[add-on key] message" into `kh1-kh2.log` (at most 2000 lines a
  session for all add-ons).
- **No data changes through v1.** Rows, abilities and finishers come from `addon.toml` only.

## 7. Finishers: what's missing (not built in tm44)

The plan the owner approved: an equipped add-on finisher is played the way an equipped KH2
finisher is, natively: a KH1 attack row and request of its own, a KH1 motion slot built from the
add-on's ANB (with its hits as KH1's motion events and its attack data from the TOML), picked in
the combo by KH2's row walk. tm44 reads and checks the finisher data and the motion file, and
stops there, because:

- KH2's moves on KH1 rows of their own (`win/kh1rows.rs` `OURS`) are a fixed table: their rows,
  requests (`0x200 + n`), attack data copies, clash rules and the motion slots built for them
  (`win/kh2slots.rs` `prepare`), the picker's lookups (`win/combomods.rs`) and per-row state are all
  sized and indexed by it at compile time. An add-on's finisher needs that table to take rows made
  at load from data.
- The ANB has to become one of KH2 Sora's motions for the slot builder (as the chest motion is
  added, `pose.rs` `add_chest_motion`), with its hits from the TOML rather than KH2's attack rows.
- KH2's row walk picks finishers by Sora's PTYA rows (`character_core::combo`): an add-on's needs a
  row in it (its `range` gives the row's kind and reach) with an ability number of its own.

**Priority when several finishers are equipped** follows KH2's: KH2 marks every row whose checks
pass and takes the first in table order whose motion isn't the one playing (`combo::select`;
`docs/kh1-anim-slots.md` 7.z.1). The planned place for add-on rows (an open question for the
owner): after KH2's own ability finishers (Explosion, Guard Break; Magnet Burst, Aerial Finish) and
before the plain last finisher (155 / 183), in load order, so KH2's own come first and an add-on
finisher replaces the plain one.
