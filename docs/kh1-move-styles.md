# Move styles (KH1 or KH2, per kind of move)

`styles.rs`: Ground Combo, Aerial Combo, Magic, Aerial Magic, each KH1 or KH2, for base Sora
only (Drive Forms keep their own KH2 movesets). Read from `kh1-kh2.ini` (`ground_combo`,
`air_combo`, `magic`, `air_magic`), switched in game from the Config menu. Defaults: combos KH2,
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

### KH2-only combo abilities make the combo KH2's (testing branch)

The tester (thread "KH2's combo row selection", m15): option (b), "(i) only the ones kh2 doesn't
have". With Finishing Plus, Negative Combo or Berserk Charge equipped (KH1 has no such rule for
its own combo), the Base Combo and Aerial Base Combo play as if set to KH2, whatever the Config
menu says: KH2's row picker and KH2's combo rules (length, finishers, Finishing Plus's chain) on
KH1's own rows (`docs/kh1-anim-slots.md` §7), not KH1's own combo. Combo Plus, Air Combo Plus and
Combo Master are KH1's own abilities too and switch nothing. `styles::kh2` reads it
(`styles::note_abilities`, fed by `menu` each time KH1's menu changes Sora's abilities); the Config
row still shows the player's setting (`styles::setting`), and taking the ability off brings the
setting back. The log says when it switches ("move style: a combo ability only KH2 has is
equipped"). Base Sora only (Drive Forms keep their own movesets). Berserk Charge isn't in the mod's
ability list yet (MP Charge comes later), so today only the first two switch it.

## Magic

Which style applies is picked when the spell is chosen (or KH1 starts it): Aerial Magic if KH2
has him in the air then, else Magic (`formmagic::base_kh2`).

- **KH1** (default): as before. KH1 casts every spell; Reflect is KH2's.
- **KH2**:
  - Fire, Blizzard, Thunder, Cure are KH2's base-form spells (`formmagic::spell_chosen`, the
    path Wisdom's spells take): the spell's MAGC row (world 0) gives base Sora's own cast
    motions (ground, air, combo finisher) in his KH2 moveset, cast through the controller (its
    combo slots, its cast windows); KH2's script `magic/<SPELL>_<level>.mag` at base Sora's magic
    sub (level × 2 − 2, + 1 for the finisher; Wisdom's are + 10, `form::magic_sub`), its effects
    and sounds, its attacks through the spell hit records with KH1's own data for the spell at
    its level (element). Level: KH1's spell level; cost: KH1's (1 MP). Cure heals a share of
    KH1's max HP by the ATKP row's power (as Wisdom's did).
  - Gravity, Stop, Aero (no KH2 version): KH1 casts the spell, drawn with base Sora's KH2 cast
    motion (Gravity on Thunder's, Stop and Aero on Cure's, the air ones in the air) and run on
    that motion's clock, input window and movement, as Wisdom's KH1-only spells
    (`takeover::command_started`'s `FormCast`, `pose::follow`, `formmagic::BASE_KH1_CAST`).
  - Aerial Magic at KH2: air casts (Reflect too) use KH2's own air physics (the cast motions'
    no-gravity frames, KH2's gravity), not KH1's fall, as Final Form's.
- **Cure** (Config > KH1 in KH2 > Combat > Magic > Cure, ini `cure_style`, KH2 by default; the
  tester, thread kh2-magic-and-reflect-native, answer 5): with the Magic styles at KH2, KH2's Cure
  (as above) or KH1's own Cure drawn on base Sora's KH2 Cure cast motion and run on its clock, as
  Gravity / Stop / Aero (`formmagic::kh2_cure`, `kh2_spell`). No effect with the Magic styles at
  KH1 (KH1's Cure whole) or in a Drive Form (their own).
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
