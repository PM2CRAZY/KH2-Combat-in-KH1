# Shared states (testing branch)

`shared.rs`, ini `shared = on` (default). This replaces the old model for base Sora. Under the
old model, KH2's controller had him whenever KH1 showed him standing, walking or running, and
KH1's own idle, walk, run and jump were drawn back over KH2's when a move style was KH1. Drive
Forms still use the old model: KH2's controller has them all through.

Every move's animation (KH1 slot, KH2 motion, owner, setting) is listed in `animations.md`.

## The rule

The user (2026-10-20): "The goal is to share the animation state and logic between both
versions while allowing KH2 to handle its own unique physics and movement properties."

The user (after s261): "currently don't share animations but should: sora base attack 2, kh2 base
attack 2 / kh1 base finisher, kh2 base finishers / glide". The ground combo and the glide are
shared moves now, as idle, walk, run and jump are.

The move styles choose the **animations** (and, for the KH2-only moves, whether KH2 borrows him
at all). Whose **physics numbers** move him is one setting for every move, `physics = kh1 | kh2`
(below, and the Physics column).

| Move | Owner | At KH1 | At KH2 | Physics KH2 | Physics KH1 |
|---|---|---|---|---|---|
| idle, walk, run, item use (Misc Animations) | KH1's states | KH1's slots | KH1's slot drawn with KH2's motion | KH2's walk / run speed | KH1's |
| jump, high jump, fall, landing (Jump) | KH1's states | KH1's slots | KH1's slots drawn with KH2's jump / fall / landing | KH2's jump arc, gravity, fall cap, early release, air drift | KH1's |
| ground combo: swings and finisher (Base Combo) | KH1's states (its rows, hit objects, timing, damage, its menu's combo abilities; KH2 borrows him for its ability moves at either style, below) | KH1's swings | KH1's swings drawn with KH2's: its second swing (slot 99) with KH2's second (152), its finisher (101) with KH2's (154), its first and third (98) with KH2's first (151) | KH2's swing travel (that swing's root motion), at either style | KH1's own travel |
| Glide | KH1's glide state (Square with `glide = square`, its held jump with `glide = jump`) | KH1's slots | KH1's glide slots drawn with KH2's glide (75 / 76) and banked glides (77 / 78) | KH2's glide speed, fall and turn rate | KH1's |
| KH1's own air attacks, air casts, launches | KH1's states | KH1's | (the Aerial Combo / Magic styles at KH1) | their own lunge / hop height / halved speed, on KH2's gravity, fall cap and drift | KH1's |
| KH2's borrowed moves (below) | KH2 borrows him | | | KH2's | KH1's gravity, fall, jump, walk / run, air steering and drag in KH2's controller; their own heights, speeds and root motion |
| Aerial Dodge | KH2 borrows him | double jump with Jump at KH1 too | the same |
| Dodge Roll (Square on the ground) | KH1 at KH1; at KH2 KH1's own roll with KH2's 209 on its request (t523), KH2's guard on KH1's states (t489), the borrow only as a fallback | KH1's roll and guard; Square held through it: KH2's Quick Run (Quick Run equipped); without KH1's Dodge Roll, Square while moving: KH2's Quick Run | KH2's roll, guard, Quick Run (with them equipped) |
| Aerial Combo | KH1's rows at KH1; KH2 borrows at KH2 | KH1's combo, KH2 borrowing him for its ability moves (below) | KH2's combo |
| Magic / Aerial Magic | KH1 casts at KH1; KH2 borrows at KH2 | KH1's spells | KH2's spells; KH1-only spells keep KH2's cast motions |
| Reflect, Drive | KH2 borrows him | always KH2's | always KH2's |
| KH1's specials, combo modifiers, summons | KH1 | unmodded | unmodded |

## How it runs

- **KH1 has him** (a hand-off of ours: `shared::active`). KH1's control step sees the pad as
  unmodded, with these exceptions (`shared::control`, `takeover::shared_control`):
  - Square on the ground with Dodge Roll at KH2, when KH2 has something for it (a dodge roll,
    Guard or Quick Run equipped). Without any, there is no borrow and the log says why once.
  - Square held since it started KH1's roll (Dodge Roll at KH1), with Quick Run equipped: once
    KH1 lets him act at or past KH2's own Quick Run check in its roll (the roll clip's event),
    or when KH1's roll is over (`shared::QUICK_RUN`).
  - Square on the ground while moving with Dodge Roll at KH1, KH1's Dodge Roll not equipped and
    Quick Run equipped (KH2's Quick Run).
  - A fresh jump in the air when Aerial Dodge is equipped (once per air time).
  - In the air past the rise with `glide = square`, the held jump and KH1's action bit (KH1's
    glide is on Square there).

  Those presses are borrows. Quick Run has no style of its own: it is KH2's move, by its
  ability. KH1's glide on Square is started from KH1's fall state (`kh1_glide_start`) and held by
  Square, at either Glide style.
- **A borrow** happens only on a frame KH1 lets him act (KH1's can-act test; no buffer):
  - the Attack command in the air with the Aerial Combo style at KH2 (`combat::exec` →
    `ATTACK_AFTER`);
  - a command of KH2's (`combat::command` → `shared::want_kh2`, then `resume_command` runs it
    once KH2 has him);
  - the button borrows above.

  The exception is one of KH2's ability moves out of KH1's own combo or knock-back
  (`combomods`, below). It is taken at the press where KH2 would use it, KH1's swing cut there.

  `takeover::shared_frame` hands KH2's controller a fresh start:
  - in the air, with his speed;
  - the borrowing press is fresh on KH2's first frame;
  - for Quick Run out of KH1's roll: KH2's Quick Run along his facing on its first frame
    (`Character::quick_run_now`), what is left of KH1's roll motion set to its idle, its slot
    not taken for a KH1 action (`CUT_LEFT`).
- **Give-back** happens after KH2's step (`shared::give_back`):
  - **To the ground:** two frames in idle, walk, run or landing, on the ground, with no combo
    going (`Character::combo_going`) and no command waiting. KH1's ground state takes over.
  - **To the air:** two frames in KH2's plain fall (a combo over or 24 frames past its last
    swing). KH1's fall state takes over, set as an air command sets it (the same fields as
    `combat::command`).
- **KH1's own moves** (its ground combo, a spell, an item, a ledge, a ladder) go on in KH1's
  states. A command started from a KH2 move keeps its cut windows until KH1 lets him act.
  KH2's controller has no glide of its own for base Sora (its level kept aside,
  `KH2_GLIDE_KEPT`, for KH2's glide numbers).
- **A jump out of KH2's move** (`JUMP_OUT`): on the ground, where KH2's move can be left, the
  press is let through to KH1's control step. KH1 jumps him, and its states have him from there.
- **The draw** (`pose::wanted`, mode 0): KH2's controller's pose while it has him; otherwise
  KH1's slot through `pose::follow`, KH2's motion where `shared::kh2_look` says the slot's
  style is KH2's (`shared::slot_kind`), KH1's own otherwise. KH2's swings are stretched over
  KH1's swing so the blade comes round when KH1's hit lands (`Play::Swing`), with KH2's root
  travel taken out of the pose (KH1 moves him).
- **The physics setting** (the user: "if its kh1 then all the animations (vanilla, and our kh2
  animations ...) will run with kh1s physics (with exceptions to drive forms) if its kh2 then all
  the animations (including vanilla animations) will run with kh2s physics instead"; then "BUG: In
  the config there's not a setting to switch between KH1 and KH2 physics"): ini `physics = kh1 |
  kh2` (KH2's without it), the Config menu's **Physics** row (after Camera Style, live, saved to
  the ini, logged "Config menu: physics ..."). `shared::physics::source()` is the one switch
  point (`kh1phys::Rule`); a Drive Form is KH2's either way. (The hair's old `physics = on / off` switch is gone, t628.) Details and every choice: `docs/kh1-physics.md`
  §7.
- **Physics KH2 on KH1's states** (`shared::physics`, KH1's state table `0x142DB4240` wrapped
  for Sora), whatever the move styles:
  - ground step: KH2's walk / run speed (KH1's walk and run only, not its actions); in KH1's
    ground swings (slots 98..101, action flag 4; KH1's or KH2's animation, `kh2_slots` too) KH2's
    swing's root travel along his facing, stretched over KH1's swing, in place of KH1's own;
  - jump: state 1's pre-step and state 2's step, KH2's arc; KH1's own air moves in state 2
    (flag 4: air attacks, casts, a launch) keep their lunge, speed and hop height on KH2's
    gravity, fall and drift (`kh1phys::kh1_arc_on_kh2`); KH1's hold (flag `0x1000`) is KH1's;
  - glide (state 0x15, registered by `0x1402B0560`): its pre-step `0x1402B06F0` turns him at
    KH2's glide turn rate; its step `0x1402B0890` moves him at KH2's glide speed
    (`(1 - a) × top × tilt + a × last`) and fall (`(height - fall height) × ratio`, capped), from
    KH2's growth numbers at Sora's Glide level (1 at least);
  - KH2's air casts in KH2's borrow: KH2's own (not KH1's fall).
- **Physics KH1**: KH1's states are KH1's (nothing above runs), and KH2's controller gets KH1's
  numbers each frame it has base Sora (`shared::physics::kh2_controller`: KH1's gravity 0.4847,
  fall cap 20, jump height from his stats `+0x10` for the jump and High Jump, walk / run `+4` /
  `+8` past 0.8 tilt, air steering `+0x14` added and drag `+0x18`), so its borrowed moves and its
  air casts (KH1's fall, `air_cast_fall` of it) move on KH1's physics.
- **Momentum lives in KH1's own fields** (`docs/kh1-physics.md`; the user: "if sora does a kh1
  air move from a kh2 air move then he gets no momentum during his kh1 move"):
  - KH2's jump arc on KH1's states is written into KH1's arc and air speed every frame (exactly:
    KH1's rise is KH2's curve), so KH1's own air move goes on from it, and KH2's arc picks up from
    KH1's after it (not from rest);
  - while KH2's controller has him, each of its steps is written into them
    (`shared::physics::mirror`);
  - KH1 taking him in the air (its air command, a give-back to the fall, its glide) goes on from
    them (`kh1_takes_air`), where the arc used to be zeroed (a fall from rest);
  - KH1's air attack out of KH2's motion keeps the faster sideways speed and the higher rise of
    its own lunge and hop and KH2's (`kh1_air_attack_started`); KH1's later swings are KH1's;
  - KH2 taking him from KH1's air state starts at KH1's air speed and arc slope
    (`kh1_velocity`), per 60 Hz frame (his travel over the frame was taken as one 60 Hz frame's:
    twice his speed at 30 fps).

## The dodge roll at KH2 (s261's "kh2 dodge roll is unusable")

The s261 log: all 28 ground Square borrows (lines 3658..3726) came after KH1's menu was
changed at line 3597 (everything but Hurricane Blast unequipped: "dodge roll 0, guard false",
quick run 0, line 3598). KH2's controller then had nothing for Square, so each borrow gave him
back two frames later in idle or run ("KH2's move is over (Run)" or "(Idle)" after each). The
borrow itself worked. KH1's own roll needs its Dodge Roll equipped too (its ability bit `0x20`
of `0x142D5F610`, set each control step from `0x140282A70 (22)`). Now Square with Dodge Roll at
KH2 borrows only when KH2 has a roll, Guard or Quick Run. With Dodge Roll equipped, KH2's roll
comes out (moving: the roll; standing: Guard).

## Frame rate (30 fps)

The user: "when the game is running at 30 fps anytime a kh2 move is used its effectively makes
the game run in slowmotion". KH2's side used to advance one 60 Hz frame per game frame. KH1's
own side scales by its frame scale (`0x1423405DC`: 1.0 at 60 fps, 2.0 at 30; the s261 log shows
1, 2 and one 4 at a hitch, in the camera's `dt 1.00..4.00` lines).

- **Whole 60 Hz steps** (`takeover::steps`, set on Sora's physics frame from the frame scale, at
  most 4, a fraction carried over): KH2's controller runs that many full steps a game frame.
  Each step runs the whole thing: the controller, his move through KH1's mover and floor, KH1's
  grabs, the animation, hits (`Strikes::strike`), Reflect and form shots, effect and sound events,
  the effects' tick, swing starts and trails, the give-back test. So no window or event is
  skipped, and KH2's 60 Hz numbers stay exact. At 60 fps there is exactly one step: as before.
- **Presses** go to the first step. Later steps see the buttons held, with no new press.
- **Once a game frame:** the platform carry and push, KH1's old position (`+0x490`), the
  touched-objects mask, and the drawn pose (`POSE`).
- **Merged across the frame's steps, for KH1's once-a-frame checks:** KH2's guard windows (as
  KH1 guard objects and for `hurt`), invulnerability and guarding (`last_out`).
- **The steps stop early** when KH1 takes him (a grab, a command) or the move is over
  (`shared::give_back`).
- **While KH1 has him:** KH2's effects tick `steps` times a frame. The cut windows of KH1's
  moves (grounded Fire's, an item's) and a form's cast clock count 60 Hz frames
  (`HANDOFF_TICKS`), as does a form cast's slide.
- **Counters in 60 Hz frames** (`takeover::TICKS`): `hurt`'s re-hit grace, and the hair and cloth
  swing (`pose`).
- **KH2's physics on KH1's states** (`shared::physics`): the arc already advanced by the frame
  scale. Its air steering's keep and drag are now taken to the scale's power.
- **KH2's motion into KH1's fields** (`shared::physics::mirror`): written on every 60 Hz step, per
  60 Hz frame (KH1 scales it by its frame scale when it moves him). KH1's physics step doesn't run
  on KH2's frames, so nothing is moved twice.

## The camera

KH1's own moves keep KH2's camera (`camera_calls = kh2`, `docs/kh1-kh2-camera.md`). KH1
holding Sora still in one of its states is not a scene for the camera. Its ledge, climbing and
pole cameras are KH2's follow camera too. Only events and KH1-only states (water, the carpet,
the Jungle Slider) hand the view to KH1's camera.

## KH2's abilities on KH1's own moves

`combomods.rs`. The user (testing s261): "when setting attack to kh1 sora is unable to use kh2s
ground combo abilities (this includes kh2s square combo modifers)", the same for the air combo,
and "sora is unable to use kh2s general modifiers (with the exception to aerial dodge)".

What was wrong: with a combo style at KH1, `combat::exec` ran KH1's own row for every Attack
while KH1's states had him, and KH2's controller wasn't asked. So KH2's abilities from KH1's
menu (the log's "KH1's menu gives KH2: ...") counted only while KH2 owned the combo (combo style
KH2): its action abilities' rows (`ComboRow::ability`), the Square moves (`SQUARE_MOVES` in
`character-core`: Finishing Leap, Upper Slash, Horizontal Slash), Negative Combo and Finishing
Plus (KH2's `max_count` / `max_finishers`), Aerial Recovery and Retaliating Slash (KH2's blow
only; KH1's own knock-back played instead), Quick Run (only through Square with Dodge Roll at
KH2). Combo Plus, Air Combo Plus and Combo Master are also KH1 abilities (ids 6, 7, 65), which
KH1's own combo reads itself; KH1's modifiers (Vortex, Sliding Dash, Hurricane Blast, ...) too.

The fix is a borrow at the press where KH2 would use the ability; KH1's own combo plays
everywhere else:

- **Attack** (`combomods::attack`, from `combat::exec`'s first branch, KH1's row about to run):
  KH2's combo is asked what it would take with its combo at KH1's count
  (`Character::would_take`: a trial on a copy of the controller, the combo carried as
  `set_combo_state` carries it, KH2's targets and KH1's R1 lock as its step sees them,
  `takeover::kh2_would_take`). KH2 borrows him when that is:
  - a row that needs an ability: an opener on a far target (Slide Dash, Flash Step, Aerial
    Dive), a step (Dodge Slash, Vicinity Break, Aerial Sweep, Aerial Spiral) or a finisher
    (Explosion, Guard Break, Aerial Finish, Magnet Burst);
  - KH2's finisher where KH1 would still swing, with Negative Combo;
  - a KH2 step where KH1 would finish, KH2's combo being longer (a Combo Plus KH1 didn't count);
  - with Finishing Plus, a press during KH1's finisher (its motion still playing): KH2's next
    finisher.
- **Square in KH1's combo window** (`combomods::control`, from `shared::control`; KH1's motion
  event flag `+0x1CC` bit 22, a swing slot 98..104): KH2's Square move at that count, if KH1's
  swing connected (the combat info's hit flags `+0x50`) or Combo Master is on, as KH2 refuses one
  after a whiff.
- **Knocked away** by KH1's own reaction (`hurt` notes a kind 4 / 5 hit while KH1's states have
  him, `combomods::launched`: in the air, from KH2's 16th frame): the jump is KH2's Aerial
  Recovery (`Character::air_recovery`), Square its Retaliating Slash.
- **Quick Run**: Square while moving with Dodge Roll at KH1, Quick Run equipped and no dodge roll
  (where KH2 runs rather than rolls): `shared::control`'s Square borrow.

The borrow (`takeover::shared_frame`, `combomods::take_want`): not only on a free frame. KH1's
swing or reaction is cut as a spell's end frames are (its action flag 4 off, its motion request
its idle). On KH2's first step (`combomods::before_step`), the combo is set at KH1's count and the
row forced (`Character::force_row`), with the press.

After the move, an Attack in KH2's combo window with the combo style at KH1
(`combat::command`, `combomods::next_from_kh2`):
- KH2's next is an ability move again: KH2 keeps the press.
- Else: KH1's next row with KH2's count. For KH1's step, `combomods::kh1_count_in` sets KH1's
  count (`+0x41`), its combo window bit and its hit flags; they are put back after it. KH1's own
  states have him from that swing on (`takeover::shared_now`).
- In KH2's finisher: its next finisher with Finishing Plus, else a new KH1 combo, as before.

With no press, KH2's combo ends and gives him back as any borrow does.

KH1's combo (static RE, `0x1402A5380`, row pickers `0x1402A4D50` / `0x1402A4AE0`):
- The count is the combat info's (`+0x14C`) byte `+0x41`.
- Outside a combo window it is set to 1. Inside one it goes up by 1 when the swing connected
  (`+0x50`, cleared by `0x1402B10E0` for each new attack object) or Combo Master (65) is
  equipped; else no swing.
- The finisher comes once the count reaches the stats' (`+0x6C`) `+0xD4` (ground) or `+0xD5`
  (air).

Unverified in game:
- that KH1's own swing stops cleanly when cut (its attack object, its trail);
- that KH1 runs its Attack step during its finisher at all (Finishing Plus needs it);
- that writing KH1's count, window and hit flags for the hand-back gives the next row;
- KH1's knock-away reaction kinds and its height for Aerial Recovery.

The log has every decision: "KH1's ground combo, swing n of m: KH2 would take ...", "Square in
KH1's combo window", "Attack in KH2's combo (count n) with the combo style KH1's", "KH1's combo
goes on from KH2's".

Overlap with the shared-moves work (`pose.rs` / `shared.rs`): `shared::Seen` gained
`combo_window`, `launched` and `moving`, and `shared::control` calls `combomods::control` first
and has the Quick Run case. Nothing in `pose.rs` changed.

## KH2's moves as KH1 slots (`kh2_slots`)

`kh2slots.rs`, ini `kh2_slots = on` (default off; or a list of KH2 motions, e.g. `152`), `docs/kh1-anim-slots.md`. Moves now: KH1's ground combo at Base Combo KH2 (requests `0xC8..0xCB` → KH2's 151, 152, 151, 154). KH1 Sora's actor gets
copies of its slot table (KH1's 118 slots and ours after them) and request table. While a move's
style is KH2's, the KH1 request it stands in for plays our slot: a KH1 motion (the KH1 slot it
replaces, retimed to the KH2 motion's length) carrying the KH2 clip's hit, combo and trail
windows as KH1's motion events, KH1's own sounds and foot effects rescaled. KH1's state, attack
row, attack object and request id are unchanged. `pose` draws the KH2 motion at KH1's time
(`actor + 0x16C`) and blends by KH1's blend clock; KH2's own sounds and effects for it fire from
KH1's clock (`kh2slots::tick`). `pose::sora_slot` and `slot_frames` report the KH1 slot it stands
in for (with the length that plays), so `shared::physics`, `combomods` and `takeover` see KH1's
slot numbers.

## Not yet

- The aerial combo, Magic and Dodge Roll at KH2 are KH2's own moves (borrows), not yet KH1's
  states drawn with KH2's motions.
- An Attack while KH2's controller has him on the ground (in its roll's recovery, its Quick
  Run) is KH2's ground combo, not KH1's.
- At Base Combo KH2, the combo is KH1's: KH2's own combo abilities (Combo Master, Negative
  Combo, Finishing Plus, Combo Plus counts) don't apply to the ground combo; KH1's from its menu
  do.
- Unverified in game: KH1's glide slots 76 and 78 (75 and 77 were seen in the s261 log; earlier builds took the glide for slots 113..116, which are attacks), the banked
  glides' direction, whether KH1's swing travel all goes through its ground step's
  displacement (if not, KH2's travel adds to KH1's), and KH1's roll's can-act frames.
- KH2's physics on KH1's states: see `shared::physics`.
