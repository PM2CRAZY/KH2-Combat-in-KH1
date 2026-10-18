# KH1's battle command menu and the Drive page (s189)

Read from the user's KH1 executable (Epic RVAs, image base 0x140000000; Steam noted where it
differs) and its `btltbl.bin`, by a helper session; what the mod does with it is in
`crates/kh1-hook/src/win/drive.rs`.

Epic RVAs (image base 0x140000000) unless marked Steam. Read statically (Ghidra decomps in
/opt/re/out/fn, capstone, btltbl.bin from local/re/kh1). Not tested in game.

## Menu structs (KH1 .data, 0x38 bytes each, menu k = 0x140529600 + k*0x38)

| k | addr | type | what |
|---|------|------|------|
| 0 | 0x140529600 | 0 | top level (Attack, Magic, Items, row 3 = Summon) |
| 1 | 0x140529638 | 1 | magic list |
| 2 | 0x140529670 | 2 | item list |
| 3 | 0x1405296A8 | 3 | summon list |
| 4 | 0x1405296E0 | 4 | party target list (Cure, Aero, items) |
| 5 | 0x140529718 | 5 | L1 shortcut menu (replaces the root while L1 held) |
| 6 | 0x140529750 | 4 | one-entry target list (entry 0x4C) |

Layout: `+0` type, `+4` greyed bits (drawn brightness), `+8` blocked bits (run refused, "can't"
sound), `+0xC` cursor, `+0x10` count, `+0x14..+0x23` 16 command ids (bytes), `+0x24..+0x33`
row brightness (drawer steps toward 0xE6 / 0x73 from `+4`), `+0x34` the entry whose submenu is
open below this level.

Chain: 0x142D543F0 [root + 7 levels, qwords to 0x142D54428], current level pointer
0x142D54450 (== &0x142D543F0 at the top). Cursor row's record 0x142D543E8 / 0x142D54430.
Chosen command 0x142D54440 (+4 target handle 0x142D54444).

## Who fills the top level

`0x140292090` (called each frame by the step `0x140291660` before its input handling):
- entry[0] (0x140529614) = 1 Attack, or 0x62 (world mode 0x142340E5C == 6, or 0x10 / 0x18),
  or 0x63 (a special target state); entries[1..3] = 0x42 (none);
- `+4` / `+8` = ~(state flags >> 4) | ~1 (bit 0 Attack from the movement state record
  0x142DB4240 + state*0x40);
- calls `0x140292330` (the "battle menus fill" the mod hooks): magic list, entry[1] = 2 and bit 1
  cleared when Sora has a spell; shortcut menu; summon list (entries 0x1405296BC = summon + 0x2F,
  count 0x1405296B8, bits 0x1405296AC/B0, availability 0x140286DB0) and **appends command 6
  (Summon) to the magic list** (greyed when all summons are, or save+0x874&4, 0x142867CD8&0x2000,
  0x142D54438&2); while a summon is out (0x14298D1A0 & 8, set by 0x140287090) it appends 0x4F
  instead (record 0x4F: type 0x4F, no submenu);
- items: entry[2] = 3 (bit 2) and the item list; party targets (menu 4); menu 6 count 1;
- tail-calls `0x140292760`: entry[3] (0x140529617) = the context command (Triangle: Talk,
  Open...; 0 when none), bit 3.

Count is static data: 4 (exe .data). Top layout 0x142859C60 has 3 row panels + the last row's
own (built by 0x140278A00 from 0x140276010) -> exactly 4 rows.

## Row 3 is not entries[3]

- Drawn by `0x140279700` case 0 -> `0x140277F40(0, ...)` -> `0x1402769D0` (layout 0x142859C60,
  x 0x10, y 0x104). Rows 0..count-2 from entries / brightness (row 0 shown as Attack at half
  brightness when entry[0] is 4 or a context type, `0x140275E40`). The last row and the cursor
  row at index 3: if top entry[1] == 2 and magic.last (0x14052964C[count-1]) is 6 or 0x4F, the
  name is magic.last and its brightness magic `+4` bit (count-1); else command 0x42 ("none").
  entries[3] (context) is drawn in its own box with the Triangle icon (0x46).
- Names: `0x1402787B0 (cmd, x, y, bright)` -> record(cmd) `+4` word -> `0x14028D820` (call at
  0x1402787FB, already redirected to `reflect::word`). Text only, no row icons.
- Confirm on row 3 (step 0x140291660, 0x140291C70..0x140291D23): if entry[1] == 2 and magic.last
  is 6/0x4F: `0x1402912F0(&magic, magic.count-1)`, then root `+0x34` = 0xF and root entry[0xF]
  (0x140529623) = magic.last (so the chain check sees root -> record(6).sub 3 -> menu 3). Else
  "can't" (sound 4). D-pad right (0x100) never opens row 3's submenu (0x140291020 excludes
  top cursor 3).
- The magic list hides its last row when it is 6/0x4F (drawer) and its cursor wraps at count-1
  (0x140291020); Summon is only reachable from top row 3.

## Records (btltbl.bin, record base file +0xA58 = 0x142D24198)

`+0` flags (4 aim at lock-on, 8 range check, 0x10 run on held), `+2` ?, `+4` word, `+6` type,
`+7` arg, `+8` submenu k (signed; > 0 opens 0x140529600 + k*0x38), `+0xC/+0xE` ranges.
1 Attack (type 1), 2 Magic (sub 1), 3 Items (sub 2), 6 Summon (sub 3, word 6, unique),
7.. spells (type 2), 0x2F.. summons (type 6, arg summon), 0x38.. party (targets),
0x42 none, 0x43 "no command", 0x4F summon-out, 0x7D Reflect (ours), 0x7E..0xFF all zero.

## Running an entry (0x1402912F0 (menu, entry))

blocked bit -> "can't" if 0x142D54440 & 0x2000000; record `+8` > 0 -> push menu k (sound 6;
type 4 resets its cursor), return 0; else (type != 0x47) checks: types 4/5/0x37 need
0x14029CE20 == 0 and actor bit 0x400 clear; type != 100 needs 0x1402BF530 == 0; state +0x70 != 1;
when 0x1402A0D30 (actor) == 0 only type 2 arg 0 (motion +0x164 0x3D/0x8A, +0x16C past a time)
or type 99 go on (no sound on these refusals), then 0x142D54440 byte0 = type, bytes1..2 = (char)arg << 8 (sign-extended,
masked 0xFFFF00), top byte flags kept; 0x142D54444 = target handle; chain popped to root;
return 1. The control step 0x1402A4090 then calls 0x1402A5380 (hooked: combat::command) while
byte0 != 0.

(Reflect, type 2 arg 7, passes the air test the same way KH1's spells do; a Drive record of
type 2 with an arg >= 8 goes the same way.)

## Input

Logical state 0x14298D4A0 (built by 0x140288F30 from the pad table 0x140528F90, 19 pairs):
`+4` held, `+8` pressed this frame, `+0xC` released, `+0x10` / `+0x14` repeats
(0x1402892A0). D-pad (raw PS2 order -> logical): up 0x10 -> **0x20**, down 0x40 -> **0x40**,
left 0x80 -> **0x80**, right 0x20 -> **0x100**. Others: Triangle 0x1000 -> 0x8 | 0x4000000,
Square 0x8000 -> 0x10 | 0x800000, R1 0x800 -> 0x400, L2 0x100 -> 0x800 | 0x400000, R2 0x200 ->
0x400000, L1 0x400 -> 0x40000, Select 0x1 -> 0x8000, confirm 0x210001, cancel 6.

Menu navigation 0x140291020 (from the step at 0x140291BCE, `&0x14298D4A0`): reads `+8`
exactly: 0x20 up, 0x40 down, 0x100 open the cursor row's submenu (`0x140292720`, not top row 3),
0x80 or bit 2 (cancel): in a submenu pop a level (sound 3; cancel also zeroes the cursor); at
the root, 0x80 alone moves the cursor to row 0 (Attack, sound 1). That is KH1's only D-pad-left
use found on the battle path (direct references to 0x14298D4A4..B4 listed in btnrefs.txt; the
player think 0x1402A4090 -> 0x14028A650 / 0x1402A59A0 read stick and other bits only).

## Steam

Steam = Epic - 0xD00 for the menu structs (0x528900 top, 0x528938 magic, 0x5289A8 summon list,
0x5289BC its entries, 0x528915 top entry[1]); chain 0x2D539F0 / level ptr 0x2D53A50; functions
+0x2190 here (0x291020 -> 0x2931B0, 0x291280 -> 0x293410, 0x2912F0 -> 0x293480, 0x291660 ->
0x2937F0, 0x292090 -> 0x294220). Steam's fill isn't byte-identical (register choice); same logic.
Every new `Rva` needs a steam_map.in entry (unmapped ones log and fall back to Epic).

## Proposed design (ours)

Page 2 differs from page 1 only in row 3, so reuse KH1's Summon path end to end:

1. **Toggle** in a pre-step hook on 0x140291660 (shared with `shortcuts::step`; one hook per
   prologue): when `[0x142D54450] == &0x142D543F0`, `[0x142D543F0] == 0x140529600` (not the L1
   menu), the menu is up (step's own gates: 0x142867CDC & 4 clear, 0x142867CD8 bits 2/14/20/27
   clear, Sora present), no summon out (0x14298D1A0 & 8), and `[0x14298D4A8] & 0x80`: flip the
   page, clear 0x80 from `+8` (KH1 would move the cursor to Attack), play sound 1
   (0x140177F60), reset menu 3's cursor (0x1405296B4) to 0. Done before the original, so the
   fill inside the same step already sees the page.
2. **Fill** after 0x140292330 (shortcuts::fill / reflect::fill, after `reflect::magic_menu`), on
   page 2: write the Drive form records (0x7E.. : `02 00 FF 00 | word | 02 arg | 00 FF 01 02 |
   0..`, arg a value KH1 has no spell for, e.g. 8 + form) each frame like `ensure_record`;
   overwrite menu 3: entries = form ids, count, `+4`/`+8` greyed/blocked bits; make sure
   magic.last == 6 (append it with its bit if KH1 didn't, e.g. no summons yet; then the
   8-row-panel fix may be needed, `reflect::eight_rows`), and set/clear its bit (count-1) in
   magic `+4`/`+8` for Drive's own availability; if top entry[1] isn't 2 (no spells) set it to 2
   with bit 1 kept blocked (else row 3 isn't drawn as Drive).
3. **Name**: `reflect::word` (0x1402787FB) answers word 6 with "Drive" on page 2 and the forms'
   word indices (e.g. 0x1FE..) with "Wisdom Form" etc. The redirect must be installed even with
   `reflect = off`.
4. **Choice**: menu 3 entry -> 0x1402912F0 -> 0x142D54440 = `02 | arg<<8`, chain back to root ->
   0x1402A5380 -> `combat::command`: route `kind == 2 && arg == form arg` to a `drive::chosen`
   before the KH1-cast queue (as Reflect).

## Wisdom Form (stage 191, `form.rs`)

- Choosing the form's entry (`drive::chosen`) asks `form` for it: refused (the "can't" sound)
  without 3 MP, while KH1 has Sora, or with its files missing. In the form the same entry reads
  "Revert".
- The change follows KH2's (kh2-bevy `drive.rs`): base Sora's Drive motion 52 (invulnerable),
  then the swap to `P_EX100_MAGF` (moveset 3, same 256-bone skeleton as Sora's, so the pose's
  retarget is unchanged): facing, lock and glide button kept; abilities = Sora's support ones,
  no action or growth ones, Quick Run at the form's level (1), every combo row of the form's
  table. Arrival motion 53. On the ground only (waits for it).
- The gauge: `(level + 3) × 600` frames, counted only in battle (KH1's battle bit). Then, on
  Revert, or knocked out: back to Sora (revert motion 4; none on a KO), his own abilities back.
- Shots: Wisdom's attack motions carry frame trigger 1 on their shot frames (151: 25, 26, 27 at
  10/20/30; finisher 154: 28..40; air 181: 41..43). Each runs the model's object script (BAR
  entry 3 of `P_EX100_MAGF.mdlx`, "wisd") with the PAX of `P_EX100_MAGF.a.us` (entry 18): a
  muzzle element bound to Sora, and the shot element (20), unbound, which the script moves with
  its attack (ATKP 859, radius 50). The hook places that effect where the script has it each
  frame (`kh2fx::spell_place`, kh2-fx `Effects::set_anchor`) and lands the attack through the
  spell hit records; a hit marks the attack hit so the script ends it. Unlocked: the nearest
  enemy within 1500 in front is the target.
- Spells in the form: magic sub + 10 (Reflect's rows exist at sub 10 and 14). From stage 192
  Fire, Blizzard, Thunder and Cure chosen in the form are KH2's (`formmagic.rs`): MAGC's cast
  motions (56..67, the form's own animations), KH1's level and cost, KH2's `.mag` script at
  sub level × 2 − 2 (+ 1 finisher) + 10 (test `wisdom_spells_run_at_their_sub`: Fire ATKP 502,
  Blizzard 248, Thunder 249 / 250, Cure a direct application to Sora), their effect packages
  (`kh2fx` 5..16) and sounds. Cure's heal is ours: ATKP power / 3000 of KH1's max HP (Cure
  sub 10..14: 708 power 800..1200 plus 709 400..600).
- Stage 192 also: Quick Run MAX (the user); the gauge drains ½ a frame out of battle (exe; the
  notes); the arrival's looping elements (100: a loop from tick 45, 11 / 12) ended when the
  arrival ends, and every form effect put out on a revert (they stayed on Sora); Wisdom's
  ground inertia (prty acceleration 0.85 / brake 0.97: he slides to a stop in his idle) and
  its motions' allow-movement range (41: the shots 0..46, the casts) in `character-core`.
- The look: `tint.rs` (`docs/kh1-tint.md`). Stage 192: Sora's PC model has the alternate mesh
  table (header `+0x10` = 0x96969696, the table behind `+0x14`); pieces picked by texture.
