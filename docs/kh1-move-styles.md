# Move styles (KH1 or KH2, per kind of move)

`styles.rs`: Ground Combo, Aerial Combo, each spell (Magic below), each KH1 or KH2, for base Sora
only (Drive Forms keep their own KH2 movesets). Read from `kh1-kh2.ini` (`ground_combo`,
`air_combo`, `magic_fire` ...), switched in game from the Config menu. Defaults: combos KH2,
magic KH1 (as before the switches).

## Combos

`stylecombo.rs`. With the combo where Sora is set to KH1 (on the ground: Ground Combo; in KH2's
air: Aerial Combo), the Attack command is not KH2's attack button:

- **Start.** KH1's command step (`0x1402A5380`, hooked as `combat::command`) runs it as KH1's
  other commands: KH2 must be able to take a command (`can_take_command`, else it waits as they
  do); in KH2's air Sora is first put in KH1's air state 2 (the air-time fields, KH2's sideways
  speed), so KH1 picks its air row. Our call of the step marks `STARTING`, so the attack row it
  runs (`0x1402A4760`, hooked as `combat::exec`) is KH1's own instead of KH2's press. KH1 then
  has him (`hand_to_kh1`, `FROM_COMMAND`), and `stylecombo::started` marks the hand-off as a
  combo's.
- **During.** Attack presses stay in KH1's command record (`take_chosen_command` leaves them) and
  its rows run (`exec`), so KH1 chains its combo, finishers and abilities as unmodded. None of the
  spell cuts (`takeover`'s `out_any`): no jump / Square / spell out of a swing; a spell chosen
  meanwhile waits for him to be free (`MAGIC_AFTER`), then runs as after any KH1 move. The draw
  is KH1's own swing (`pose`: no KH2 swing over KH1's combo slots 98..104, no stale KH2 pose at
  the start).
- **End.** As after a KH1 special: once KH1 lets him act, the stick, a jump, Square or (in the
  air) his fall hand him back to KH2 (KH2's fall and landing after an air combo). KH2's combo
  starts over afterwards (no `COMBO_ON` carry). Attack while KH1 has him free is KH1's again if
  the style where he is (KH1's air state 1 / 2 or the ground) is KH1's; else it is KH2's press
  and KH2 takes him (mixed styles: a KH1 ground combo then a KH2 air combo, or the other way).
- Any other hand-off (`hand_to_kh1`) clears the combo mark; it holds only while `HANDOFF` and
  `FROM_COMMAND` do.

Unverified in game: that KH1's command step needs nothing more than its state to pick the air
row; that KH1's can-act test opens only after a swing's recovery (so the stick doesn't hand him
back between swings); KH1's own hit reactions mid-combo.

### KH2-only combo abilities follow the combo settings (testing branch)

t488 (the thread "KH2's combo row selection", m15) made Finishing Plus, Negative Combo and Berserk
Charge turn the Base / Aerial Base Combo to KH2's whatever the Config menu said. The tester (m19):
"the ability should not ignore the combat settings". Now `styles::kh2` is the setting alone, and at
KH1 the abilities act on KH1's own combo:

- **Negative Combo**: KH1's length (the stats' `+0xD4` ground / `+0xD5` air, KH1's Combo Plus /
  Air Combo Plus already in it) less one a copy, at least 1, as KH2's length takes it off its own.
  Our wrap of KH1's row picker (`kh1picker::shorten`) lowers the byte only while KH1's picker runs,
  so KH1's own finisher test marks the finisher a swing sooner; `combomods::kh1_combo` reads the
  same length. Not applied to KH1's swimming combo (state 0x18, its Attack step reads the byte
  itself) nor to a spell's combo-finisher test (`formmagic`).
- **Finishing Plus**: KH2's Finishing Plus window on one of KH2's ability finishers playing on KH1's
  rows (Explosion, Guard Break, Aerial Finish, Magnet Burst ...; `kh2slots::finisher_window`) opens
  KH1's combo window while another finisher is left (one plus the copies, KH2's count), so KH1's
  Attack step counts the press on past its length; KH1's own picker then takes a finisher (a KH2
  ability finisher by KH2's row checks, or KH1's own), run on a KH1 row whose request isn't the one
  playing (`combomods::attack`, `next_finisher`). KH1's own finisher motions have no such window, so
  no chain follows KH1's own finisher (an open question to the tester).
- **Berserk Charge**: not in the mod's ability list yet (MP Charge comes later); its refusal of
  finishers is in KH2's row checks (`Character::ability_row`).

The log says when one comes or goes ("move style: a combo ability only KH2 has is equipped ... it
follows the settings").

### What the KH1 setting still takes from KH2 (testing branch, checked in the code)

With base Sora on KH1's own states (`shared`), every Attack goes through `combomods::attack`
whatever the style. At Base / Aerial Base Combo KH1:

- KH1's picker (`0x1402A6EE0` ground, `0x1402A6C70` air) picks the row; KH2's row selection doesn't
  replace it (`Kh1Combo::here` false for plain rows), and KH1's request table entries point at KH1's
  own motions again from the next press (`kh2slots::redirect` with `styles::kh2` false).
- KH2's drop-ahead test is off (`test_drop_ahead(false)`), so at a ledge KH1's own row 2 (its swing
  that walks him off a drop ahead) runs, not KH2's hop (row 1); KH2's edge stop only acts while one
  of our KH2 slots plays.
- KH2's ability moves come only out of KH1's own row picker (the tester, m19: "modify kh1s so that
  it can choose explosion and other moves during its kh1 combo"; `docs/kh1-anim-slots.md` §7.z). KH2's
  selector isn't asked at this setting any more: KH1's picker decides the count, swing or finisher by
  KH1's own length, its own ability rows and its target; where it took one of its plain rows, KH2's
  equipped ability rows of that kind join its choice by KH2's own row checks, and the one taken runs
  on KH1's rows as before (KH2's motion, KH1's row, hits and timing). KH2's plain swings, KH2's
  combo length and its walk over the whole table play no part (t537's filter is gone).
- Finishing Plus and Negative Combo work on KH1's own combo at this setting (the section above).

## Magic

Per spell since the testing branch's per-spell rows (the tester, thread
kh2-effects-in-kh1s-own-effect-system m19: "each magic should have its own settings of which magic
you can choose, e.g fire: kh1 | kh2 and that would change what gets casted, we'll remove ground
magic and change air magic to air physics"). The Magic and Aerial Magic rows are gone; the ini's
old `magic` / `air_magic` / `cure_style` / `kh1_cast_*` keys are read only as the new keys'
defaults, so an ini from before keeps what it cast (`settings`).

Config > KH1 in KH2 > Combat > Magic:

- **Fire, Blizzard, Thunder, Cure** (`styles::Kind::Fire` .. `Cure`, ini `magic_fire`,
  `magic_blizzard`, `magic_thunder`, `magic_cure`; KH1 by default): which game's spell base Sora
  casts, on the ground and in the air alike (`formmagic::kh2_spell`, `base_kh2_spell`). At KH2,
  KH2's spell on KH1's cast state (`formmagic::native`, `kh1_spell_start`: KH2's script, MAGC
  motion, effects, sounds, KH2's MP cost scaled, `docs/kh1-magic-native.md`), or borrowed by
  KH2's controller in a state KH1 casts nothing in. At KH1, KH1's command step casts KH1's
  spell as unmodded (its script, effects, heal, cost), on the animation its row below picks.
- **Animations** (its own list, so both lists keep KH1's full-size rows on screen; the tester's
  config-menu-rework answer "(a) for now if more options get added to a menu you can shrink the ui
  to fit"): Blizzard, Thunder, Cure, Gravity, Stop, Aero (`Kind::BlizzardAnim` .. `AeroAnim`, ini
  `anim_blizzard` .. `anim_aero`; KH1 by default): the cast animation of **KH1's** spell
  (`formmagic::anim_kh2`). The tester: "you can also change the animation of magic with the
  exception being reflect, and fire": Fire has no row (its animation is its spell's), Reflect's is
  always KH2's (KH1 has none).
  - A spell cast KH2's way always plays KH2's cast motion: KH2's spell scripts end the cast when
    Sora isn't in it on their first frame and release on its magic-cast event
    (`docs/kh1-magic-native.md` §4). (Open with the tester: KH2's spell on KH1's animation.)
  - KH1's Blizzard, Thunder, Cure at KH2: KH1's requests (0x37 / 0x38 / 0x39, air 0x84 / 0x85 /
    0x86) point at our slots of KH2's cast motions (`kh2slots::redirect`, by `cast_spell`), and
    KH1's own script runs on them (`kh1_spell_start` leaves KH1's spells alone): KH2's
    animation, its combo / cut windows (`cast_follow`) and its motion's own sounds; KH1's spell,
    effects, timing and cost. **Assumption**: KH1's spell scripts time their shots by their own
    frames, not by the cast motion's events (KH1's cast motions carry no spell events: §1 of
    `docs/kh1-magic-native.md`, **Checked (files)**), so the shot comes out at KH1's time.
  - KH1's Gravity, Stop, Aero at KH2: as before (Gravity on Thunder's cast motion, Stop and Aero
    on Cure's, the air ones in the air, run on that motion's clock, input window and movement:
    `takeover::command_started`'s `FormCast`, `pose::follow`, `formmagic::BASE_KH1_CAST`). The
    debug menu's KH1 Spells page shows and sets these three rows (KH1 cast = KH1).
- Costs: base Sora's KH2 spells cost KH2's MP scaled onto KH1's (`mpcost`, `docs/kh1-magic-native.md` §6).
- The spells' effect packages are loaded with `drive = off` too (they were only with Drive on).
- If a spell's KH2 file or MAGC row is missing, KH1 casts its own.

## Hit Reactions (testing branch)

Config > KH1 in KH2 > Combat > Hit Reactions (ini `hit_reactions`, KH1 by default; the tester,
thread soras-hit-reactions-native). At KH2, base Sora on KH1's states plays KH2's flinches on
KH1's own reaction requests (`kh2slots::REACTIONS`: ground 10..13, air 14 / 15); KH1's hit pass
still runs every reaction (no-damage time, push, voices, HUD, MP, tallies). At KH1, KH1's
reactions as they are. Drive Forms and KH2's borrowed moves keep KH2's controller's reaction.
Notes: `docs/kh1-hit-reactions-native.md` §9.

## Recoil (testing branch)

Config > KH1 in KH2 > Combat > Recoil (ini `recoil`, KH1 by default; the tester, thread
kh2-recoil-and-parry-system, answer 4: "if you switch to kh1 all moves including kh2 uses kh1's
rules exactly and vice versa"). The rules when Sora's attack meets an enemy's attack or guard.
At KH1, KH1's: its priorities, its recoil animations (0x6E / 0x6F / 0x70) with its push and 8
frames without damage. At KH2, KH2's: base Sora on KH1's states plays KH2's recoil (41 on the
ground, 43 in the air; `kh2slots::RECOILS`) with KH2's push of 50 or hop, no protection; KH1's big
recoil (0x6F) keeps its animation (answer 1). Drive Forms and KH2's borrowed moves keep KH2's
controller's recoil animation in both. Notes: `docs/kh1-kh2-recoil.md` §6.

The Guard Repel row (Movement, t536) is part of this row now (the tester: "we need to also combine
guard repel and recoil into the same config"): what base Sora's KH2 guard does on a block follows
Recoil, KH1's own block at KH1, KH2's repel (42, 200 back, 80 frames, Counterguard) at KH2. The ini
`guard_repel` is no longer read (`docs/kh1-kh2-recoil.md` §6.6).

## Jump and Glide

`stylejump.rs` (ini `jump_style`, `glide_style`; Config rows Jump, Glide). Base Sora only.

- **Jump at KH1.** KH2's fresh jump press on the ground while free (idle, walking, running,
  landing) runs KH1's ground state's own jump press handler (`0x1402CA3F0`, state 0's `+0x38`:
  its ground test `0x1402A0D30`, then the jump start `0x1402AD860`: jump or high jump motion,
  state 1, air time 0), KH2's ground speed carried into KH1's air speed (`+0x3C0`). KH1 has him
  (`hand_to_kh1`, mode `JUMP`) through its rise, fall, ledge grabs and landing. In the air KH2
  takes him back only for one of its moves, once KH1 lets him act: an Attack with the Aerial Base
  Combo at KH2 (KH2's aerial combo, the press carried), a fresh Square (Aerial Dodge, or KH2's
  glide on Square), a fresh jump press while falling for KH2's glide on the jump button, or a
  command (magic, items, a Drive: it waits in `PENDING_COMMAND` and runs once KH2 has him, as
  KH2's per the Aerial Magic style). With the Aerial Base Combo at KH1 an Attack is KH1's own air
  combo (the hand-off becomes that combo's, `stylecombo`). Back on the ground: KH1's landing, then
  KH2 again as after KH1's other moves (the stick, Square, Attack, a jump; a jump there is KH1's
  again). KH1's own glide in its jump: only with Glide at KH1 (the jump button is left to KH1's
  control step then, `kh1_keeps_jump`; with Glide at KH2 it stays stripped).
- **Glide at KH1.** KH2's own glide is off (`abilities.glide = 0`). In KH2's jump, a fresh jump
  press while falling (KH2 in its air or jump state) puts him in KH1's air state 2 as an air
  command does (air-time fields, KH2's sideways speed) and starts KH1's Superglide
  (`0x1402B05C0`) when he has it (`0x140282A70 (4, 0xB)`), else Glide (`0x1402B0530`, ability 3),
  both through `0x1402B0BA0` (motion 0x71, state 0x15; nothing below its height over the floor:
  then KH2 keeps him). KH1 has him (mode `GLIDE`) while it glides; back in its fall, KH2's fall;
  down on the ground, KH2's landing.

Unverified in game: that KH1's jump rises from where KH2 left him (`+0x3B4` set to his height);
KH1's air speed from KH2's ground speed; that a fresh jump press while falling matches how KH1
starts its own glide (KH1's state 2 takes the press, its state 1 doesn't); Superglide's own input
(KH1's other glide path, `0x1401CD926`, prefers Superglide when he has it, as here); hits taken
during KH1's jump or glide (KH1's reactions; KH2's Aerial Recovery doesn't come out of them).

## Dodge Roll

`styleroll.rs`. With Dodge Roll at KH1 (base Sora; Drive Forms keep KH2's):

- **KH1's roll.** KH1's per-frame button handler (`0x1402A59A0`) rolls on Square in its ground
  state 0, out of deep water (`0x1402BF530`), with the Dodge Roll ability (bit `0x20` of
  `0x142F5F610`, Steam `0x142F5EC10`; `0x10` is Guard, and with both the stick's tilt against
  the stats' `+8` picks): it faces him along the stick (`0x140295E20 (actor, dir)` the angle,
  `0x140295C80 (actor, angle)` sets it) and starts the roll `0x1402C7CB0` (rolling flag
  `0x100000`, motion `0xDC`, `0x14029D8A0 (0x40000000)`). Its untouchable frames are its
  motion's events (flag `0x40`).
- **Start.** A fresh Square with the stick on the ground, KH2 having him free (idle, walk, run,
  landing), runs that roll (`takeover::style_roll_start`) and hands him to KH1. Without KH1's
  Dodge Roll ability (or in deep water) the press stays KH2's (its roll by its own abilities, or
  Quick Run). Square standing still is KH2's guard as before.
- **During.** KH1 keeps him while motion `0xDC` plays (its request pending on the first frames):
  no hand-back on the stick. An Attack KH1's command step takes meanwhile (KH1's own window) goes
  by the Base Combo style: KH1's combo from there, or KH2 takes him at once with the press for
  its combo. A command (magic, an item, a Drive) waits for the roll's end, then is KH2's (by the
  Magic styles).
- **End.** As after KH1's other moves: the stick, a jump, Square or an Attack hand him back.
- Quick Run: KH2's roll turns into it with Square held through the roll; KH1's roll doesn't (no
  such thing in KH1). Quick Run itself is untouched.
- KH1's own roll sound / voice play from its motion; `kh1voice::dodge_roll` is on KH2's roll only,
  so they don't double.

Unverified in game: the facing call's angle argument; that motion `0xDC` shows in `+0x164` from
the roll's second frame (logged for its first four frames); that KH1's command step takes Attack
during its roll at all (else an Attack comes after it, by the hand-back's rules).

## Stage 242 changes

- Glide at KH1 is on Square, held while falling (in KH2's air and in KH1's jump), not the jump
  button: the jump press in the air is KH2's Aerial Dodge (in KH1's jump KH2 takes him for it).
  While KH1 glides, Square held is KH1's jump-held bit in its control step (its glide is a hold).
  One try per hold of Square.
- KH2's glide with the style back at KH2: its level is kept aside while KH1's glide is on
  (`KH2_GLIDE_KEPT`; Sora's abilities are only set again when KH1's menu changes).
- Dodge Roll at KH1: KH1's ability bits are at `0x142D5F610` (Steam `0x142D5EC10`), not
  `0x142F5F610`, so the roll never opened.
- Battle Quotes (KH1 | KH2, ini `battle_quotes`, default from `kh2_voice`): every KH2 motion voice
  is loaded and played only with it at KH2, so the row switches in game. On every form too. It
  is on a third Config page (our rows go 8 to a page).

### Dodge Roll at KH1: the model on Sora (roll-decouple)

The user (s244): "soras model decouples when inputting a kh1 dodge roll". The log: during KH1's
roll, and on KH2's first frames after it, the skeleton pass's placement (KH1's model matrix, its
translation at actor `+0x260`) sat off his position (`+0x10`): KH1's per-frame motion update
(`0x14029CF10`, from the actor loop `0x1402965A0`) builds that matrix with its motion's root
compensation (the roll moves the actor, the matrix is pulled back by the root's travel and the
roll's root bone puts him back), and at the roll's end the pull lingers for some frames. KH2's
pose drawn there (and the pose we last drew, blending into KH1's) sat up to ~300 behind him, then
snapped forward. `pose::on_actor`: in the game the placement's translation is his position
whenever it is off it by 0.5..1500 (logged, 30 times). Also no stale KH2 pose (`POSE`, fresh for
0.15 s) over KH1's roll's first frames (as for KH1's combos): KH1's roll from its first frame,
blended in from the pose last drawn.

## Misc animations

`styles::Kind::MiscAnims` (ini `misc_anims`, default `kh1`; Config row "Misc Animations", before
Battle Quotes), base Sora only. At KH2, KH1 still runs the action (its effect, its timing, the
item's logic); only the drawn animation is KH2's (`pose::follow`, a forced `Play::Motion`, no
stale KH2 pose over its start):

- **Item use** (KH1's item command, type 3, `takeover::item_use_now`): KH2's motion 9 (`A110` in
  P_EX100.mset; 25 frames at 30 fps = 50 game frames; the right hand rises overhead around frames
  20-30; a voice trigger at 0 and sound 305 at 10). Picked from the moveset by its shape (no name
  in the file); KH1's use runs ~52 frames (`ITEM_CUT` 51), so they line up without stretching.

Not mapped (no clean KH2 counterpart, or not identified yet): summon calls (KH2's summons share
the Drive change, motion 52, but KH1's summon command isn't told apart here yet), examining /
picking up, KH1's other field actions. Guard, the dodge roll and hit reactions already have their
own settings or are KH2's.
