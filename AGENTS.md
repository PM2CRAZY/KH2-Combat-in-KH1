### 2026-10-26: Stage 649 (testing branch: KH1's spells in states KH1 casts nothing in: KH1's rule)
- **KH1's spells in states KH1 casts nothing in: KH1's rule.** The tester (what-is-still-missing, m6): "(b) if you have magic set to kh2 those magics should be 1:1, the reverse is true regardless of the animation." KH1's cast `0x1402A3600` casts only in states 0, 6, 0x18, 2 and 8; its menu `0x140293480` refuses state 1, deep water (`0x1402C16E0`) and a failed may-act test `0x1402A2EC0` (Checked (decomp)). Gravity, Stop and Aero with their animation row at KH2 were borrowed by KH2's controller in the other states; now KH1's command step runs them unmodded, casting nothing as KH1 does (`combat::command`, logged "KH1's rule, no borrow"). KH2's spells in those states are unchanged (still a borrow; open question). Not checked in game. docs/kh1-magic-native.md §4p.

### 2026-10-26: Stage 648 (testing branch: KH2's air combo on KH1's flight attack)
- **KH2's air combo on KH1's flight attack.** The tester (what-is-still-missing, m6): "(a) if there's a more elegant implementation to let the animations work as flight air attacks e.g if the game has a different animation for sora when attacking in the flight state then we'll just give kh2 more animations, if not then find how it works and give kh2 the ability". KH1 has no flight attack animation of its own: in flight (state 8) KH1's Attack step `0x1402A7510` runs the air picker's row `0x1402A6C70` + 3 (rows 19 / 20 / 21: requests 0xCE / 0xCD / 0xCC, the air combo's, lunge 8), and the row runner `0x1402A68F0`'s case for 19..21 levels the direction, moves him at the lunge and queues the flight motion 0x75 (Checked (decomp); rows Checked (files), btltbl `+0xA9F0`). At Aerial Base Combo KH2 the air picker hook makes KH2's air pick in state 8 as in state 2; a KH2 air move on a row of ours returns 16 / 18 so KH1 runs its flight row 19 / 21 (`combomods::flight_pick`), and `combat::exec` runs that row with our row copied into its place for the call (request, attack data, trail / effect / voice bytes; KH1's lunge kept; `kh1rows::run_as`, put back after). Anything else stays KH1's own flight row. With shared states on, the old `FLY_ATTACK` borrow was already unreachable (Checked (code)). Assumptions: our requests resolve in Neverland's motion set; KH1's runner reads the row only at its start. Steam only. Not checked in game. docs/kh1-anim-slots.md §7.z.10.

### 2026-10-26: Stage 647 (testing branch: The fall's loop range no longer leaks into the next motion)
- **The fall's loop range no longer leaks into the next motion.** The tester (kh2-glide-in-kh1s-glide-state, after m7): "I also noticed that neutral glide resets its animation wrong when setting jump to kh2 and doing a glide out of jump". The trace (f113654..f113834): the hover 208 (100 frames) looped 13..32 after our baked KH2 fall, the fall's own loop range (Checked (trace)). `kh2slots::loop_range` runs from KH1's pose pass, which KH1's apply (Steam `0x1402A0350`) calls on the incoming motion before writing `+0x168` and after `0x14029FC40` cleared `+0x160` bit 6: it still saw the fall, set its 12..32 and bit 6 again, and a blended start keeps the range it finds (Checked (decomp)). Now it does nothing while `+0x160` bit 0 is set (a request queued or being applied: set by `0x14029F7C0`, cleared by the update `0x14029F0A0` after the queue step, Checked (decomp)); the fall's range is set one frame later. Also covers a 205 glide out of the fall. Not checked in game. docs/kh1-anim-baking.md §7.2, docs/kh1-glide-native.md §8.12.

### 2026-10-26: Stage 646 (testing branch: Glide bank to bank as a fresh blend)
- **Glide: bank to bank as a fresh blend from the pose on screen.** The tester (kh2-glide-in-kh1s-glide-state, after m7): "I'm still seeing the same phenomenon, I sent you a trace ... if you find a fix for both just go ahead and implement it." A bank switch passes his facing (206 → 205 for one frame → 207, Checked (trace)), so the second change came while KH1's blend still ran. KH1's apply (Steam `0x1402A0350`, from `0x14029FC40`) chains such a request (`0x140393B30`): its source becomes the last blend's target, 205's frame 0, not the pose on screen; its fresh path `0x140394080` blends from `+0x1B8` / `+0x1B0`, which the blend pass `0x140392560` keeps as the drawn pose; KH2's `0x1403C6DC0` → `0x1403C86A0` never chains (Checked (decomp) both). `glide::fresh_blend`: when the pre-step's request is queued (`+0x178` ≥ 1) during a blend (`+0x170` > 0), `+0x170` is set to 0 so KH1 takes the fresh path; the motion still starts at frame 0, held through the 16-frame blend. Assumption: KH2's blend source is the drawn pose (the OG rig port, not re-traced). Glide motions 205..208 only. Not checked in game. docs/kh1-glide-native.md §8.11.

### 2026-10-26: Stage 645 (testing branch: Glide out of the Aerial Dodge on KH2's frame)
- **Glide out of the Aerial Dodge on KH2's frame.** The tester (aerial-dodge-native, after m4): "air dodge right now you seem to not be able to cancel it into a glide as fast as you can in kh2, can please check and see against the c steam decomp if its letting you? if you see the reason why then implement it." Steam KH2's glide start `0x1403A81F0` (PLAYER vtable `+0x100`) needs a free bit `+0x18C & 0x1C` (`0x1403BFB30`); 202's only free range is id 1 from frame 4 (Checked (files)). KH2 asks 202 at time 0 in the input step; its motion update `0x1403C6740` skips 0→1 to 2 and runs before the input step (`0x1403BFD30`: update, then PLAYER `+0x20` → `0x1403A89A0`; vtable by RTTI, Checked (exe)), so Square glides 3 frames after the dodge press (Checked (decomp)). Ours was 5 (the tester's traces: the action flag clearing on the fifth frame, Checked (trace)), as KH1 plays 0, 1, 2, 3, 4 and its control step runs before its animation. `combomods::dodge_clock` puts the slot time at 1 on the first frame after KH1 applies our dodge request (KH2's skip, as `glide::kh2_clock`), and the dodge slot's "may be left" starts one frame before KH2's free frame (`kh2slots::INPUT_LEAD`): 3 frames after the press, as KH2. Assumption: KH2's motion step is 1 per 60 Hz frame. Other KH2 slots' free frames keep the one-frame order lag. Not checked in game. docs/kh1-aerial-dodge-native.md §9.

### 2026-10-26: Stage 644 (testing branch: Gravity and Stop as the combo's finisher)
- **Gravity and Stop as the combo's finisher.** The tester (kh2s-combo-row-selection, after m29): "for gravity and stop, there's a phenomenon in kh2 where those magics count as a finisher so the combo row skips straight to the finisher, if you have a finishing plus equipped you can combo into another finisher ... that would be applied for kh1 gravity and stop as well if the config is set to can combo". Ground Gravity (0x3A) / Stop (0x3B), base Sora, on KH2's cast animation always and on KH1's with Magic Combo KH2 (`formmagic::finisher_spell`): the step is the combo's length (or one past it in a Finishing Plus window, `note_finisher_step`), taken as a finisher at KH1's spell start (`take_step`, `combomods::cast_step`); KH1's count 0 after it unless Finishing Plus leaves another finisher. Their KH2 cast slots get no combo window (built as finisher casts); the Attack / spell checks and KH1's combo window take only a Finishing Plus window placed by share from KH2's Thunder finisher cast 63 (Gravity) / Cure's 68 (Stop), range 50 42..78 (Checked (files); placement an Assumption; `finisher_spell_window`, `kh2slots::finisher_share`). KH2 has no Gravity or Stop: the rule is the tester's. Not checked in game. docs/kh1-magic-native.md §4c.4.

### 2026-10-26: Stage 643 (testing branch: Magic Combo: KH1's grounded casts in the combo)
- **Magic Combo: KH1's grounded casts in the combo.** The tester (kh2s-combo-row-selection, after m29): "make a config for magic that allows you to combo out of grounded magic like kh2 ... for kh1 magics logic you just apply kh2s fire combo logic on the first shot and the repeated shot animation ... go ahead". New row Combat > Magic > Magic Combo (KH1 | KH2, ini `magic_combo`, default kh1; `styles::Kind::MagicCombo`). At KH2 (base Sora, ground, no form): KH1's own Fire / Blizzard / Thunder / Cure / Aero cast takes its step of the combo (`formmagic::note_step` at the command, `take_step` at KH1's spell start `0x14028F140`); KH1's own cast animation (requests 0x36, 0x3D, 0x37, 0x38, 0x39, 0x3C, Checked (files)) gets KH1's combo window (instance `+0x14` bit 22) from KH2 Fire 56's share (frame 60, `kh2slots::combo_share`) to its end (`formmagic::kh1_cast_window`, from `kh2slots::frame`), so KH1's menu and Attack step (`0x1402A2EC0`, `0x1402A7510`, Checked (decomp)) take an Attack or a spell. KH1's quick Fire unchanged. Gravity / Stop not yet. Assumptions: the window by share; KH1's script casts whatever motion follows. Per frame: one flag write while a KH1 cast plays. Not checked in game. docs/kh1-magic-native.md §4c.3.

### 2026-10-26: Stage 642 (testing branch: KH2's combo window on its casts, range 35 by motion)
- **KH2's combo window on its casts, range 35 by motion.** The tester (kh2s-combo-row-selection, after m29): "you can combo kh2 magic into each other faster than going from a magic into attack so I need you to look into it because its not 1:1", "if I use reflect ... the move doesn't let you combo", "kh2 blizzard can't combo into anything". Steam KH2's range step `0x14040D810`: range id 35 sets the combo window bit (`+0x18C` 0x400, as range 11) only when its parameter equals the motion playing (the motion player's `+0x28`, set from the queue in `0x1403C6740`), Checked (decomp); Sora's casts carry copies of other casts' ranges (Reflect 71 Fire's, Cure 65 Thunder's), Checked (files). `WindowKind::ComboMagic` now carries its motion (kh2-data); `kh2slots::combo_kind` makes KH1's combo window on our cast slots, and the Attack and spell checks in `combat::command`, KH2's: Fire 56 from 60, Blizzard 59 from 36, Thunder 62 from 42, Reflect 71 34..46, Cure 65 none. Before: spells from every range 35's start (Fire 34, Cure 42, Reflect to its end, Blizzard never), Attacks only in range 11. Drive Forms' controller unchanged (`ComboMagic(_)`). Not checked in game. docs/kh1-magic-native.md §4c.1.

### 2026-10-26: Stage 641 (testing branch: Finishing Plus counted at the KH2 setting)
- **Finishing Plus counted at the KH2 setting.** The tester (kh2s-combo-row-selection, after m28): "if you set combat to kh2 for both settings you can attack out of your finisher indefinitely, it needs to behave exact like kh1". `combomods::finisher_chain_open` returned "always" at the combo style KH2, so with Finishing Plus every KH2 finisher opened KH1's combo window again (`kh2slots::finisher_window`) and chained without end. KH2 counts its finishers in its Attack and command check (Steam KH2 `0x1403D6CA0`: the finisher window only while `+0xD75` is under the maximum, Checked (decomp)), not in its row selection, so both styles now count the same way (`FINISHERS` against one plus Finishing Plus's copies), and a press that still reaches KH1's Attack step in a finisher with none left is no attack (`attack_row`), as at the KH1 setting. No addresses changed. Not checked in game. docs/kh1-anim-slots.md §7.z.9.

### 2026-10-26: Stage 640 (testing branch: KH2 spells' attacks in KH1's own collision pass)
- **KH2 spells' attacks in KH1's own collision pass.** The tester (what-is-still-missing, after m5, "(c)"), first step. Each attack a KH2 spell script makes (Fire, Blizzard, Thunder, Cure, Reflect's counter / burst, Wisdom's shots) is a free-standing KH1 attack object made with KH1's constructor `0x1402B2DC0`, as KH1's spell scripts make theirs (script call 23; first bytes checked, Checked (exe)); KH1's collision pass collides it (t638's entry hook puts in the script's shapes). Its data: the KH1 spell's entry or Sora's swing / finisher row's copy, clash type 6, KH2's spark group and the package's effect list. KH2's hit record kept: the count from the group picks the ATKP row (`bodyhits::before_hit`, `kh1rows::kh2_atkp`), per-target re-arm. Hits go back to the runtime from KH1's contact. Changes: spell hits now take KH2's knock-back / launch numbers (`scale_record`); a hit reaches the script up to a step later. The scripts themselves and Reflect's barrier meeting are still ours. Steam only. Not checked in game. docs/kh1-magic-native.md §4o.

### 2026-10-26: Stage 639 (testing branch: No KH2-controller borrow for the button fallbacks)
- **No KH2-controller borrow for the button fallbacks (Steam).** The tester (what-is-still-missing, after m5, "(b)": base Sora never on KH2's controller). `shared::control`'s three fallbacks (Square for KH2's dodge roll / guard / Quick Run when `guard_press`, `roll_press` and `quickrun::start` refused, e.g. KH1's deep water `0x1402C16E0`; the air jump when `combomods::aerial_dodge` refused; Square held through KH1's roll when `quickrun::start` refused) leave the press to KH1 as unmodded, as t630 did for the combo. Logged once each. Epic unchanged. Remaining base-Sora borrows listed in the doc. Checked (code); not checked in game. docs/shared-states.md "The last button borrows gone".

### 2026-10-26: Stage 638 (testing branch: KH2's body hits in KH1's own collision pass)
- **KH2's body hits in KH1's own collision pass.** The tester (what-is-still-missing, after m5): "(b), (c) … go ahead". Each KH2 hit window on Sora's body (Vicinity Break 1869, Explosion 911 / 912, Magnet Burst 1900 / 1901 / 1903, also Counterguard's and Finishing Leap's) is a KH1 attack object of Sora's, made with KH1's row maker `0x1402B3270`, tied to his request and to a KH1 attack group of its own (1..3, our slot's hit-window events, `kh2slots::body_windows`). KH1's collision pass (Steam `0x1402B4460` → `0x1402B47A0` → `0x1402B3800`) collides it: `0x1402B3800` is hooked (prologue `40 55 56 57 48 81 EC 80 00 00 00`, Checked (exe)) to put KH2's ellipsoid / column (radius, own half height) and KH2's sweep in as KH1 entries, each tested with KH1's `0x1402B3BE0` (Checked (decomp)). Contact, record, damage and reaction are KH1's own; the ATKP comes through `kh1rows::kh2_atkp`. Per-target re-arm at the ATKP interval (KH2's), KH1's object clock left off. Shapes Checked (files) `P_EX100.mdlx`; KH2 bone 67 → KH1 70. `takeover::kh2_hits` keeps only the body guards and the Magnet pull (still ours). Steam only. Assumption: offsets on bone shapes in KH1's bone frame (none of Sora's has one). Not checked in game. docs/kh1-anim-slots.md §7.z.8.

### 2026-10-26: Stage 637 (testing branch: No fallback: every KH2 pick on a KH1 row of its own)
- **No fallback: every KH2 pick on a KH1 row of its own.** The tester (kh2s-combo-row-selection, after m26): "1. remove the fall back, and make them their own animations if they aren't already". t630's `kh1_own_row` is removed; nothing falls back onto KH1's rows or buttons. Finishing Plus's generic next-finisher rows get a slot of every KH2 finisher of their kind with a row of their own (`kh2slots::prepare`: ground 154 / 155 / 166 / 165, air 183 / 184 / 194 / 197), so when KH2 repeats the finisher playing (a PTYA row whose `sub` is its own id passes the walk, Steam KH2 `0x140402C50`, Checked (decomp) at t374) one of the two rows is always free; KH1's finisher row 8 / 16 no longer plays KH2's motion (`next_finisher`). At Base Combo KH2 with Aerial KH1, KH2's plain hops (rows 0 / 1) play on their own rows (`combo_row`). A pick whose slot didn't build at load is no attack, and a Square / jump move without its row takes the button and does nothing, logged (`own_rows`; `kh1_slots = off` keeps the old plain path). Unchanged: Square with no Square move passing is KH1's roll / guard. Assumption: a finisher on the generic row uses that row's attack data (154's / 183's), its hit numbers still KH2's. 7 more slots baked at load. Not checked in game. docs/kh1-anim-slots.md §7.z.6 (t631, written now) and §7.z.7.

### 2026-10-26: Stage 636 (testing branch: KH2's landing baked into KH1's landing slot)
- **KH2's landing baked into KH1's landing slot.** The tester (same thread, after m3), and m1's "same as kh1 for now" for the landing lock. KH2's landing (motion 5, 26 frames, Checked (files)) on KH1's landing request 7 (slot 11 as template), with the High Jump's 0xC on the same slot (`ALIASES`; both entries `0x200B`, Checked (files)); KH1's player step asks it with action flag 4 on touchdown and queues the idle (Checked (decomp)). The template's events stay at KH1's own frames (style Jump), so KH1's landing lock ("may be left" at 12), dust and sound stay KH1's. No loop point (KH2's lies past its end). `pose::play_for`'s landing stays as the fallback. Not checked in game. docs/kh1-anim-baking.md §11 (and §12, the audit of what is still drawn over KH1's pass).

### 2026-10-26: Stage 635 (testing branch: KH2's idle baked into KH1's idle slots)
- **KH2's idle baked into KH1's idle slots.** The tester (same thread, after m3): "just make sure every animation is baked and on the native system". KH1's idle request 0 (entry `0x4000`, kind 4: the resolver `0x1402A01E0` plays the slot in the field and slot + 1 in battle; Checked (files + decomp)) gets two slots of ours (`kh2slots::CYCLES` now with a slot count): KH2's field idle (motion 0's field variant, 180 frames) on slot 0 and KH2's battle idle (300 frames, Checked (files)) on slot 1, style Misc Animations. Everything that asks KH1's idle (standing, the `0x40000000` action-over request after landings, items, casts, reactions; scripts) now plays KH2's idle in KH1's own pass. The bake takes motions over 254 frames (`bake::reduce` loosens a channel's tolerance until its keys fit a byte; test `long_channels_fit_a_byte_of_keys`); the battle idle comes back within 0.36 units offline on the user's files. `pose::play_for`'s idle stays as the fallback. Not checked in game. docs/kh1-anim-baking.md §10.

### 2026-10-26: Stage 634 (testing branch: Foot IK off)
- **Foot IK off.** The tester (baking-the-rest-of-the-base-animations, after m3): "lets actually have the foot iks off we'll work on this later when we do soras cloth animations". New ini key `foot_ik` (default off) gates both foot placements: KH1's own leg IK and ground lift on the baked slots (t632; `kh1_leg_ik` now needs `foot_ik = on`, so the baked legs are plain FK again) and our own `feet::place` on clips still drawn over KH1's pass (run with `active = false`: it eases out, then leaves the pose alone). Code kept for the cloth work. `feet::place` is the hook's only caller of KH1's floor ray `0x1401910E0` (Checked (code)); KH1's own motions keep KH1's own lift. Not checked in game. docs/kh1-anim-baking.md §9.

### 2026-10-26: Stage 633 (testing branch: The bake's helper channel count)
- **The bake's helper channel count.** A fix to t632 (baking-the-rest-of-the-base-animations, m2: "(a) go ahead"). KH1's pose pass (Steam `0x140391C70`) samples as many helper channels as the motion header's `+0x20` says (Checked (decomp)); `bake::baked` rebuilt the list (travel, the foot helpers' channels dropped and ours added) but left `+0x20` at the template's count, so KH1 sampled too few channels (the foot helpers' last keys never read: legs off the baked ankles) or read past the list. Now written with the list's length; `bake::tests::bakes_a_chain` (failing on testing for this) passes. Found by a second helper's version of the same step. Not checked in game. docs/kh1-anim-baking.md §7.5.

### 2026-10-25: Stage 632 (testing branch: KH1's own leg IK on the baked slots)
- **KH1's own leg IK on the baked slots.** The tester (baking-the-rest-of-the-base-animations, after m2): "(a) go ahead" (option (a): "build KH1's own leg IK onto the baked slots (feet on slopes and stairs), but don't bake idle and landing yet"). Every one of Sora's 118 KH1 motions has the same leg rig: hip 16/42 aimed at foot helper H9/H8, thigh mode-2 two-bone IK, ankle 19/45 at the IK's end turned against the model, foot 20/46 copying the helper. The bake (`kh1_data::bake::baked(.., kh1_legs)`, `kh1_data::legs`) keeps those joint words on our slots and keys the helper (under matrix 0) to the baked ankle and foot, the hip's rotation x to the baked thigh's roll, the shin's rotation x and the ankle's model turn to the baked ones, so KH1's own bone pass (Steam `0x1403900B0`, solver `0x14038FC20`) rebuilds the baked legs on flat ground and lifts them onto slopes and stairs with its own ground lift (Sora's lift list in his model header: thighs 17/43, height 17.434). Our foot IK (`feet.rs`) still runs only on drawn clips (idle, landing, glide). Ini `kh1_leg_ik` (default on). Checked (decomp + exe): the pass, the solver, the matrix conventions. Checked (files): the rig in all 118 motions, the lift list, KH1's replayed pass within 0.19..0.69 units of the pose on ten baked motions (`examples/bake_legs.rs`). Assumption: the instance's lift bits are on during our slots as during KH1's own motions (t348 trace). Per frame: nothing of ours. docs/kh1-anim-baking.md §8.

### 2026-10-25: Stage 631 (testing branch: Every KH2 combo row on a KH1 row of its own)
- **Every KH2 combo row on a KH1 row of its own.** The tester (kh2s-combo-row-selection, after m25): "3. (a) make sure you patch in any features missing in kh1's row so that it can handle it 1:1". `kh1rows::OURS` gains 20 combo rows (`Ours::combo`, one per KH2 motion: 151, 152, 153, 162, 163, 164, 169, 170; 154, 155, 166; 181, 182, 192, 191, 196; 183, 184, 194, 197), each with its own request 0x200 + n and slot, the template row's trail / effect / sound / voice bytes and a copy of its attack data with KH2's ATKP reaction, knock-back, launch and clash rules; no lunge (btltbl rows at +0xA9F0, Checked (files)). KH1's picker returns the row itself (`combomods::combo_row`); KH1's Attack step runs the picker's return in states 0 / 2 (Steam `0x1402A7510`, Checked (decomp)). KH1's plain requests 0xC8..0xCE are no longer re-pointed per press (fewer bakes at load). What KH1 reads from `+0x164`: the Attack step's Fire out of a 1st / 3rd swing sees the template request for that call (`combat::command` wrapper); event opcode 322 (Dive to the Heart's tutorial) gets the template request (`probe::event_op`), both Checked (decomp). Assumption: `0x14039A4A0`'s effect pick (200 / 0xD2..D6) isn't Sora's. Change in play: KH2's plain swings and finishers land with KH2's knock-back. Not checked in game. docs/kh1-anim-slots.md §7.z.6.

### 2026-10-25: Stage 630 (testing branch: No KH2-controller borrows in the combo)
- **No more KH2-controller borrows in the combo.** The tester (kh2s-combo-row-selection, after m25): "2. (b)". The borrow path (`combomods::want` / `take_want` / `lent` / `before_step`, `takeover::shared_frame`'s ability case) is removed. An Attack whose KH2 pick no KH1 row can play (slot not built, no free row for a further Finishing Plus finisher) runs KH1's row its picker chose (`kh1_own_row`; at the KH2 setting, picked with KH2's length); a finisher there counts on Finishing Plus's chain. Square moves, Retaliating Slash and Aerial Recovery with no KH1 row leave KH1 its own Square / jump. `Run::Kh2` is now `Run::Nothing`. Assumption: the picker row at the KH2 setting is the one picked with KH2's length. Not checked in game. docs/kh1-anim-slots.md §7.z.5.

### 2026-10-25: Stage 629 (testing branch: Zantetsuken follows KH1 Ability Chance)
- **Zantetsuken follows KH1 Ability Chance.** The tester (kh2s-combo-row-selection, after m25): "no it should change on the toggle we have like the other moves, if you have ability chance off then it should be 100 percent like the other moves, if not then 30% chance is back". `combomods`' Zantetsuken step takes `Kh1Rule::Chance` like Stun Impact and Gravity Break (t589's always-roll rule is gone). With the Tweak on, it is KH1's third 30% roll on the same random number (Steam `0x1402A6EE0`, Checked (decomp) at t580/t589); with it off, it passes whenever its single-target group's checks pass. The Tweaks help text names it. Not checked in game. docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 628 (testing branch: No hair switches)
- **The hair switches out again.** The tester (missing-sleeves-animation, after m21): "2.(b)", i.e. take the `hair_physics` / `kh2_hair` switches out again without changing anything else. Both ini lines, their settings fields, the old `physics = on / off` fallback and its migration in `save_physics` are gone; `pose` always runs KH2's own animation of the hair, hood, sleeves and keychain, as the switches' defaults (on / on) did. Nothing else changed: `jiggle.rs` still swings the chains KH2 doesn't pose this frame (the dream weapons' and wooden sword's keychain), as it does with the switches on. No addresses. Checked (code: every reader of the two fields); not checked in game. docs/kh1-secondary-motion.md "Switches".

### 2026-10-25: Stage 627 (testing branch: Jump and High Jump baked into KH1's takeoff)
- **Jump and High Jump baked into KH1's takeoff.** The same thread (plan step 4). `kh2slots::JUMPS`: KH2's jump (3, 26 frames) on KH1's takeoff 0x4 (template slot 8) and its High Jump (201, 64 frames) on 0x9 (template 12), style Jump, held at the end until the switch into the fall. KH2's jump start (`0x140422630` / `0x1403B59B0`, Checked (decomp)) sets the fall motion to 4 for both, so 201 is cut at apex − 5 as the plain jump is (the exe's rule, not the OG rig's "plays to its end"; 201 has no range id 49, Checked (files)). KH1's apex slots 9 / 13 are never asked at Jump KH2. KH1's takeoff sounds and foot effects stay at KH1's own frames (the takeoff is frame 4 in both). Assumption: KH1's takeoff bit 16 holds at the end. Not checked in game. docs/kh1-anim-baking.md §7.3.

### 2026-10-25: Stage 626 (testing branch: The fall baked, and KH2's switch into it)
- **The fall baked, and KH2's switch into it.** The same thread (plan step 3, after m1: "go ahead"). `kh2slots::FALL`: KH2's fall (motion 4) on KH1's fall requests 6 and 0xB (`ALIASES`; both slot 10, Checked (files)), style Jump. KH1 loops it from KH2's loop point 12 of 32 through KH1's own loop range (`+0x160` bit 0x40, `+0x288` / `+0x28C`; `0x1402A0350`, `0x14029F0A0`, setter `0x14029FA10`, Checked (decomp)). At Jump KH2, KH1's apex step (Steam `0x1402AF630`: its apex base+5 with blend 6.0 at `+0x3B0`+6 ≥ `+0x3B8`, then its fall queued, Checked (decomp, exe)) is wrapped (prologue `40 53 48 83 EC 20`, Steam only) to run KH2's rule (KH2 `0x1403CB680` → `0x1403CB610`: the fall motion `+0x118` = 4 with blend 8.0 at apex − 5, at once on an early release; Checked (decomp)): KH1's fall request with blend 8, no apex motion. Early releases ask it from `shared::physics`. `pose::follow_baked` keeps the baked frame's KH2 pose, so a drawn clip after a baked one blends (the drawn landing out of the fall; also baked walk / run into the drawn idle, which snapped). Assumption: KH2's arc clock and blend are in 60 Hz frames. Landing and idle still drawn over (answer 1 open). docs/kh1-anim-baking.md §7.2.

### 2026-10-25: Stage 625 (testing branch: Item use baked into KH1's item step)
- **Item use baked into KH1's item step.** The tester (baking-the-rest-of-the-base-animations, after m1): "3. (b) kh2 voice plays if the toggle for kh2 voices is on … go ahead". `kh2slots::ITEMS`: KH2's item use (motion 9) on KH1's item requests 0x3E / 0x3F (templates 58 / 59), Misc Animations KH2. KH1's command step (Steam `0x1402A3DB0`, type 3: blend 8, the idle queued with `0x40000000`, the item's task `0x1402C2760` not tied to the motion, Checked (decomp)) plays it in its own item step. KH1's "may be left" is KH2's free state (frame 36 of 50, Checked (files)); the end is KH1's queued idle. KH1's item sound is left out; KH2's sound 305 at 10 always plays (`kh2sound::allowed`), its voice 3 only with KH2 battle quotes. `takeover::item_use_now` ends with KH1's item slot instead of the 51-frame clock. Not checked in game: the heal's timing against frame 36. docs/kh1-anim-baking.md §7.1.

### 2026-10-25: Stage 624 (testing branch: Finishing Plus after a finisher cast)
- **Finishing Plus after a finisher cast.** The tester (kh2-magic-and-reflect-native, after m10): "2. (a) go ahead". Steam KH2 `0x1403D6CA0` takes an Attack, and from the input step `0x1403D6FC0` a spell (command kind 2), in the finisher window (range id 50, `+0x194` bit 0x20) while the finisher count `+0xD75` is under the maximum `[+0x5C0] + 0x202`; both counts go up by one (Checked (decomp)). The ground finisher casts' range 50: Fire 57 54..84, Blizzard 60 46..76, Thunder 63 / 68 42..78, Reflect 72 54..84; air casts none (Checked (files), `P_EX100.mset`). Base Sora's native casts and Reflect are now noted as combo steps (`combomods::cast_step`: `LAST`, `FINISHERS`; `formmagic::note_step`'s `CHAINED`). A ground finisher cast with a finisher left (`combomods::finisher_left`: under 1 + Finishing Plus) keeps KH1's count `+0x41` at its step instead of 0; `kh2slots::finisher_window` opens KH1's combo window there only while one is left; `kh2slots::cast_finisher_open` lets `combat::command` take an Attack or a spell (items stay range 11). The next finisher is picked as after a finisher swing; a spell there is the next finisher cast. Assumptions: the maximum is 1 + Finishing Plus; KH1 restarts a cast request already playing. Drive Forms unchanged (answer 1). Not checked in game. docs/kh1-magic-native.md §4c.2.

### 2026-10-25: Stage 623 (testing branch: KH2's table walk without a controller copy)
- **KH2's table walk planned without a controller copy.** The tester (combo row selection m24), step 2 of the native plan. In `character-core`, `Character::plan_press` is KH2's press step as a pure function of a `PressState` (counts, hit, carried combo, air, facing, forced row, host motion, sideways speed): count / finisher step, lock turn, selector walk `combo::select` (Steam KH2 `0x140402D20`), forced-row and Square rules, hop (`0x1403D78D0`) or lunge. `try_attack` is `plan_press` applied; `would_take_launch` / `would_take_row` (and so `ability_row_id` / `normal_finisher`) build the press state from KH1's data and run the plan instead of cloning the controller and pressing Attack on the clone. Checked: unit test `host_trials_walk_the_table_as_the_copy_did`, 34,560 cases both ways, same row and hop. Assumption: `try_attack` unchanged by the refactor (read and the 67 sim tests, not a frozen copy). The PTYA table and Sora's numbers are still read from the KH2 Character object. Expected: no change in play. docs/kh1-anim-slots.md §7.z.4.

### 2026-10-25: Stage 622 (testing branch: KH2's pick inside KH1's picker)
- **KH2's whole pick made in KH1's picker at the KH2 setting.** The tester (combo row selection m24): "(a)" / "go ahead", step 1 of the native plan. KH1's Attack step `0x1402A7510` asks its ground picker `0x1402A6EE0` (state 0) or air picker `0x1402A6C70` (states 2 / 8) and runs the returned row with the row runner `0x1402A68F0`; each has only that caller and state 1 runs no row, so the picker always runs on the same frame just before the row it picks (Checked (decomp), Steam). `kh1picker::at_kh2` now makes KH2's pick once (`combomods::kh2_press`, returning a `Press`: row, length, Negative Combo, hop, chained), runs KH1's picker with KH2's length and its ability finisher bits held back, then the t612 walk (`kh2_finisher_walk`, now `Option<i32>`); where KH2's pick stands, `kh2_pick_in_picker` returns the KH1 row that plays it (`kh2_row`, moved from the row runner; dash distance at `dir+4` cleared when it replaces rows 0 / 1) and keeps the pick for the row runner (`KH2_PICK`: same frame, row, air). `combomods::attack` takes the kept pick instead of asking KH2's selector again and no longer swaps the row afterwards. One KH2 trial per press instead of two. Expected: no change in play; not checked in game. docs/kh1-anim-slots.md §7.z.3.

### 2026-10-25: Stage 621 (testing branch: Aerial Finish and Magnet Burst follow Physics again)
- **Aerial Finish and Magnet Burst follow the Physics row again (t617 undone).** The tester (air-physics-for-kh2-magic-aerial-finish-and-magnet-burst m3): "I misunderstood, abilities should follow the main physics config." t617's code is reverted: `shared::physics::{AIR_PHYSICS_MOVES, AIR_MOVE_AT, air_physics_hop, air_physics_move}` and the `combomods::air_hop` note are gone, `kh2_hop` / `air_step` take `kh2()` again for these two moves (air casts still `cast_kh2()`), and the Air Physics (`0x7F00_0190`) and Physics (`0x7F00_011A`) help texts are back to their pre-t617 wording. The research stays in docs/kh1-magic-native.md §4i, rewritten: KH2 plays both moves inside ACTION_JUMP like its casts (row hop `0x1403D78D0`, step `0x1403CB360`, Checked (decomp)); which Config row covers them is our choice. Checked: a reverse of t617's code files; not in game.

### 2026-10-25: Stage 620 (testing branch: Sleeves: back to t375)
- **Missing sleeves animation: back to t375.** The tester (missing-sleeves-animation, after m20): "I want you to revert changes made in this thread to "> t375 ..." and bake the animations for me. we'll figure out a way later, keep the debug menu frame and frame count changes". Reverted, by intent per commit (other threads' edits in the same files kept): t385–t388, t405–t407, t411, t412, t453, t490, t491, t499, t500, t519, t552, t569, t592, t595, t596, t613, t615, t616. Back from t375: KH2's sleeve flaps on KH1's four sleeve chains plus hair, hood and keychain from KH2's keys, live and baked; the `hair_physics` / `kh2_hair` switches and `jiggle.rs` for chains KH2 doesn't pose; the coat and shorts on t375's rules; the thigh roll by the knee's direction. Kept: t389 (KH2's walk / run baked into KH1's slots; later work builds on `kh2slots::CYCLES`), t593's Frame Advance panel (its cloth line removed), t594 (Frames row), t614 (Animation label). Removed files: kh1-data `cloth.rs`, `clothsim.rs`, their local tests and examples, kh1-hook `win/clothlive.rs`. The bake (`pose::Baker` → `kh2slots::bake_one` → `kh1_data::bake`) is built at load from the player's own files, so every baked slot carries t375's motion with nothing stored in the repo. Checked (git history); not checked in game. docs/kh1-secondary-motion.md "Back to t375".

### 2026-10-25: Stage 619 (testing branch: Medium Hits on the ground and in normal air)
- **Medium Hits only on the ground and in normal air.** The tester (soras-hit-reactions-native m3, answer 3): "(b)". KH1's hit pass (Steam `0x14029BBD0`) picks its reaction function by Sora's movement state `+0x70`: the ground row for 0, 1, 0xC, 0x1C, 0x20, 0x26, 0x18, 0x19; own rows for flight 8, swimming 0x11, 0x1D, 0x1E / 0x1F and 0x27; the plain air row otherwise (Checked (decomp)). `hitreact::medium_hits` (kind 4 → 3 before the pass) now runs only in the ground and plain-air states; flight, swimming, Atlantica's water 0x18 / 0x19, the carpet 0x1D, the Jungle Slider 0x1E / 0x1F and 0x27 keep KH1's kind 4 (`medium_state`). Answers 1 and 2 needed no code (Battle Quotes already decides the hurt line; the flight flinch's animation already follows Hit Reactions through `kh2slots::redirect`). Assumption: 0x18 / 0x19 counted as swimming from earlier stage notes; 0xC / 0x1C / 0x20 / 0x26 unidentified, kept as KH1's ground. docs/kh1-hit-reactions-native.md §12, §16.

### 2026-10-25: Stage 618 (testing branch: Glide: KH2's blend clock)
- **Glide: KH2's blend clock on the glide's motions.** The tester (kh2-glide-in-kh1s-glide-state, after m6): "go ahead with any changes you need to make it 1:1". Both games hold a new motion at its start through a blend (KH2's motion update `0x1403C6740`: time `+0x4C` held while the blend clock `+0x44` runs to `+0x48`; KH1 Steam `0x14029F0A0`: `+0x174` / `+0x170` / `+0x16C`), so the bank restart is KH2's own (Checked (decomp), and the tester's log). The differences: KH2 starts a blend's clock at 2 (`0x1403C8A40`) and asks the glide's motions from the action update after that frame's motion update (vtable `+0x28`, `0x1403B2DA0` → `0x1403B4780` → `0x140423A70`, run by `0x1403BFD30`; Checked (decomp, exe vtable / RTTI)), while KH1's apply `0x1402A0350` starts `+0x174` at one frame step; and KH2 turns a time stepping from 0 to 1 into 2 (`0x1403C6740`). `glide::kh2_clock`, in the glide pre-step, for 205..208 only: on the first frame after KH1's apply `+0x174` = 2; on the first frame after a blend ends at time 0 (60 fps) `+0x16C` = 1, which KH1's update makes 2. Left: the request frame still draws KH1's 1/16. Assumption: KH2's motion step is 1 per 60 Hz frame. No new hooks. Logged (first 12). docs/kh1-glide-native.md §8.10.

### 2026-10-25: Stage 617 (testing branch: Aerial Finish and Magnet Burst follow Air Physics)
- **Aerial Finish and Magnet Burst follow the Air Physics row.** The tester (air-physics-for-kh2-magic-aerial-finish-and-magnet-burst m1, answer 1): "(b)". KH2 plays both moves (motions 194 / 197) inside its jump state like its air casts: the combo row's hop `0x1403D78D0`, then ACTION_JUMP's step `0x1403CB360` with its hold, running clock and fall cap (Checked (decomp)). `combomods::air_hop` notes the pick (`shared::physics::air_physics_hop`), and `kh2_hop` / `air_step` take `cast_kh2()` instead of `kh2()` for these two moves (`air_physics_move`: the motion playing, or its hop asked in the last 2 frames); the off-arc rule casts get at Air Physics KH1 under Physics KH2 covers them too. At KH2: KH2's hop on KH2's gravity, the hover with the clock running, then the drop at 16 a frame; at KH1: KH1's gravity, the hover freezing KH1's clock, KH1's fall. Help texts of Air Physics (`0x7F00_0190`) and Physics (`0x7F00_011A`) updated. Drive Forms unchanged (answer 2). Assumption: the move is told by KH2's motion id on our slot plus a 2-frame window for its hop. Not checked in game. docs/kh1-magic-native.md §4i.

### 2026-10-25: Stage 616 (testing branch: Cloth: the coat calmer)
- **Cloth physics: the coat much calmer.** The tester: "the bottom part of soras coat ... moves way too much, kh1 soras coat is way less pronounced then that". KH1's own field run moves each hem tip at most 0.3-4.6 units from its mean place against the chest (rms 0.2-2.7) (Checked (files)); t592's coat physics, driven by KH1's run in an offline copy, moved the tips up to 5.7 on a stop, 5.5 on a start and 4.6 in a 90° turn. New coat settings: stiffness 0.02→0.06, damping 0.30→0.40, weight 0.25→0.10, cap 40°→20° (1.5 / 1.5 / 0.8; the stride and bob response ¼ to ⅖ of t592's). `Settings::TUNED`; settings id 0 / Reset mean these numbers. The tester's own tuned values weren't in the trace (Cloth category off), so not applied. Assumption: KH2's own run's excitation not evaluated. docs/kh1-secondary-motion.md "The coat calmer".

### 2026-10-25: Stage 615 (testing branch: Cloth: links' middles clear the thighs)
- **Cloth physics: each link's middle kept out of the thighs.** The tester: "sora at the peak of his run around frame 11 is clipping through his shorts". t569's clearance rules were tested only at each link's tip, so with a knee up a link could pass through the thigh between two clear points. Each link's middle is now held to the same rules (each point with its own bind-distance cap), the link swung about its start to clear it. Assumption: which part clipped at frame 11 (coat, belt or shorts link over the rising thigh) isn't known; not checked against KH2's run frame by frame. docs/kh1-secondary-motion.md.

### 2026-10-25: Stage 614 (testing branch: Frame Advance panel: Animation)
- **Frame Advance panel: the frame line reads Animation.** The tester: "I assume field frame run is the animation count, change it to animation so its understandable". The panel's line "Frame: field run 12 / 40   Loop 3" now reads "Animation: field run, frame 12 / 40, loop 3" (`fapanel::lines`). docs/kh1-frame-advance.md.

### 2026-10-25: Stage 613 (testing branch: Coat hangs from the chest)
- **Cloth physics: the coat hangs from the chest.** The tester (missing-sleeves-animation, after m19): "the part of the coat next to soras necklace doesn't move with the rest of the coat so the mesh stretches". t592 put each coat root (72, 248..276) back on the spine base 71 at KH1's run relation, but the retarget turns 71 only 67% from KH2's pelvis toward KH2's chest while the chest 78 (the jacket by the necklace) turns fully, so the coat's top ring sheared against the jacket in KH2's run. In KH1's field run 71 and 78 stay rigid to each other and no coat root is keyed (only r+1 / r+2) (Checked (files)). The coat's carrier is now the chest 78, each root held on it at its bind relation (as the retarget's `ATTACHED` drew it before t592); the joints below swing as before. The trace's Cloth body bones gain 78. Assumption: the jacket band by the necklace is skinned to the chest (skin weights aren't read). `kh1-data/src/clothsim.rs` (`CHEST`, `Chain::held`), `clothlive::BODY`, docs/kh1-secondary-motion.md.

### 2026-10-25: Stage 612 (testing branch: KH1 finishers at the KH2 setting)
- **KH1's ability finishers at the KH2 setting, in KH1's picker.** The tester (m23): "I noticed I can't do any kh1 finisher on kh2 combat settings", "patch features into kh1's engine". At Base / Aerial Base Combo KH2 the row runner replaced KH1's ability finisher rows with its plain row whenever KH2's pick was plain (`kh2_row`, t390), so KH1's moves never came. Our hooks on KH1's pickers (`0x1402A6EE0` / `0x1402A6C70`, `kh1picker::at_kh2`) now also run at the KH2 style: KH2's pick at this press first (`combomods::kh2_press`); KH1's picker runs with its ability finisher bits held back (`HELD_GROUND`) and KH2's combo length on the length byte it reads (stats `+0xD4` / `+0xD5`, test `len <= count`, Checked (decomp)); when KH2's pick is a finisher the t580 walk runs (`kh2_finisher_walk`: KH2's table order, KH1's twins in their KH2 places, KH1's moves on their KH1 rules via `Kh1Rules`, no dash exception), and a KH1 move that passes is the row KH1's picker returns, run whole (`take_kh1_take`). Finishing Plus: `kh1_finisher_window` also opens on KH1's finishers at the KH2 setting by KH2's count, so KH1 → KH2 and KH2 → KH1 finishers chain. Cost: one more KH2 trial per Attack press at this setting. Assumption: KH1's moves keep their KH1 rules at the KH2 setting. docs/kh1-anim-slots.md §7.z.2.

### 2026-10-25: Stage 611 (testing branch: Group rows count every enemy at KH2)
- **KH2 setting: group rows count every enemy while locked on.** The tester (kh2s-combo-row-selection m23, answer 1): "(a)". KH2's selector `0x140402D20` runs its kind-2 pass `0x1404029F0` on every target its iterator `0x1403BDAC0` returns; the pass only sets the row's bit in the selector's mark array (`+0x60`) and keeps no target, and the hop and lunge (`0x1403D78D0`) are measured to the character's own primary target `+0xDC8` (Checked (decomp)). `Character::try_attack` (the KH2-setting trial) no longer keeps only the locked foe (stage 207's filter is gone), and `combo::select` aims a kind-2 row at the primary target instead of the one that passed it, so the "lunges at the foe beside him" problem stays fixed the way KH2 fixes it; unlocked play changes the same way. Test `group_row_passes_on_any_target_and_aims_at_the_primary`. docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 610 (testing branch: KH2's teamwork block)
- **kh2-magic: KH2's teamwork block shared by every script (Reflect's chain).** The tester (m9), and a correction of t572's assumption. Steam KH2 `0x1403F6DF0` (`trap_teamwork_alloc`) keeps one block per key (table `0x142AE75D0`, count `0x142AE7650`): size + 1 zeroed bytes the first time, then byte 0 set to 1 and the same block (Checked (decomp)). Reflect's script (key 0x10002) ends the previous barrier element stored at +4 at release and stores its own, adds its count to +8 when its barrier ends, and only the newest cast clears the block and fires the counter, tiered by the chain's total (Checked (files)). kh2-magic gave each cast its own block; `Magic::teamwork` now holds the shared bytes, synced into and out of each cast's memory around its run; `reflect.rs` no longer resets the chain note and counts on a chained cast. Test `reflect_chain_shares_the_teamwork_block` (needs extracted files; not run here). docs/kh1-reflect-spell.md, docs/magic-model.md.

### 2026-10-25: Stage 609 (testing branch: Effect lengths in 30 Hz ticks)
- **kh2-magic: effect elements live their length in 30 Hz ticks.** The tester (m9): "reflect is still sending its shots on the first hit". The traces (f42073 / f42322 / f42330) show every barrier attack standing exactly 27 frames, the counter's element and the 574 hits the frame after (Checked (trace)). Reflect's script keeps its barrier while element 0 lives (`trap_effect_is_alive` loop), then counters (Checked (files), `REFLECT_3.mag`). Particle-data lengths are effect ticks, which KH2 runs at 30 Hz (`0x1402C1D80`, Checked (decomp)), but `Effect::alive` compared game frames with ticks, so every element died at half its length while KH1's effect engine still drew it. `effect::FRAMES_PER_TICK` = 2; ages are compared in ticks. Test `effects_live_their_length_in_30_hz_ticks`. Side effect (1:1): every kh2-magic script that waits on an element now gets KH2's length (Blizzard's shard flies twice as long as before). docs/kh1-reflect-spell.md.

### 2026-10-25: Stage 608 (testing branch: Attack out of a cast only in its combo window)
- **Attack out of a KH2 cast only in its combo window.** The tester (m9, answer 2): "(a)". Steam KH2 `0x1403D6CA0` takes an Attack only in the combo window (range 11, `+0x18C` bit 0x400), the finisher window (range 50, `+0x194` bit 0x20, Finishing Plus left) or a free state; the reaction-command window (range 14) is only for reaction-command rows (`0x1403BA070`). Checked (decomp). In `P_EX100.mset` only Blizzard 59 has range 11 (from frame 36); the finisher casts 57 / 60 / 63 / 68 / 72 have range 50; no cast has a free state. Checked (files). `kh2slots::cast_follow`: KH1's combo window on our casts now covers range 11 plus range 35 (the magic window), not range 14 to the end; new `kh2slots::cast_attack_window`; `combat::command` refuses Attack (silently) and items (KH1's "can't") outside range 11; KH2's controller (Drive Forms / borrows) takes an Attack in a cast only in range 11. Test `attack_out_of_a_cast_only_in_its_combo_window`; the `magic_sora` / `spell_mid_combo_is_a_step_of_it` test clips now carry a combo window. Not built: Finishing Plus in a finisher cast's range 50. docs/kh1-magic-native.md §4c.1, kh1-anim-slots.md.

### 2026-10-25: Stage 607 (testing branch: KH1 spells: no jump or Square cut)
- **KH1 spells: no jump or Square cut.** The tester (kh2-magic-and-reflect-native m9, answer 1): "(a)". `takeover`'s hand-off path (a KH1 spell started out of one of KH2's moves, and a Drive Form's KH1 spell) let a jump or Square end the cast once the spell was out (s168's "other cut"); that cut is removed, so jump and Square wait until KH1 lets him act. Attack or another spell still cut the cast as before (`attack_cut` / `magic_cut`). KH2 takes a jump or the action button only in a free state (Steam KH2 `0x1403D6FC0` / `0x1403BFB30`), which no cast has. Checked (decomp). docs/kh1-anim-slots.md.

### 2026-10-25: Stage 606 (testing branch: Tweaks: Enemy Weight)
- **Config: Tweaks > Enemy Weight.** The tester (kh2-knockback-on-enemies m1, answer 1): "(b) but make it a toggle in the config menu". Combat > Tweaks gets a second row, Enemy Weight (Off | On, text ids `0x7F00_0193` / `0x7F00_0194`), switching `knockback::weight` live and saved as `enemy_weight` (default on). On: KH2's knock-back × KH1's per-enemy weight (t602-t605); Off: KH2's numbers for every enemy. docs/kh1-config-menu.md §6.y.

### 2026-10-25: Stage 605 (testing branch: KH2's air flinch)
- **KH2's air flinch in KH1's enemy air flinch.** The tester (kh2-knockback-on-enemies m1, the plan's step 3). A new Steam hook on `0x1402B8B90 (actor, record, float front)` (third argument in XMM2), prologue `48 89 5C 24 18 48 89 74 24 20 57 48 83 EC 60` (Checked (exe)). After KH1's runs (its motion 0x4E / 0x4F, hop 20, drift 4 × weight), for a KH2 record KH1's arc fields take KH2's 50 hop and the flight is KH2's: the hop on KH2's jump curve with a constant drift of push distance × 0.048 (× weight with Enemy Weight on) along the record's direction (KH2 `0x1403D4800`: 50.0 and sstm `+0x38`, Checked (decomp)). Assumption: the constant drift follows `character_core::npc`'s model. Not done: KH2's turn of the flinched body (`0x1403D3840`). docs/kh1-launch.md §4.

### 2026-10-25: Stage 604 (testing branch: KH2's push in KH1's flinch push)
- **KH2's push in KH1's enemy flinch push.** The tester (kh2-knockback-on-enemies m1, the plan's step 3). A new Steam hook on `0x1402B7E60 (actor, record)`, prologue `48 89 5C 24 18 57 48 83 EC 40` (Checked (exe)); KH1's runs first, then for a record a KH2 hit made (`knockback::note_record` from `combat::scale_record`, fresh 250 ms) KH1's push speed `+0x3FC` becomes KH2's distance × 0.1 (KH2 `0x1403D4960`): knock-back 1, or 200 when it is 32767, halved for 0xC / 0xD; a plain flinch (group 2) gets none; × stats `+0xC4` with Enemy Weight on. Before each enemy physics step (`takeover::physics`, Steam `0x14029A3C0`) KH2's ×0.9 a frame decay replaces KH1's ×0.8 in KH1's own push step `0x1402C7070` (KH2 `0x1403D4570`). Checked (decomp) both games. Left to KH1: enemies of weight 0, KH2 blows on enemies KH1 can't launch, KH1's own pushes, spells' hits (`spell_strike`). docs/kh1-launch.md §4.

### 2026-10-25: Stage 603 (testing branch: KH2's blow flight on KH1's air state)
- **KH2's blow flight on KH1's enemy air state.** The tester (kh2-knockback-on-enemies m1, the native plan's step 2). The `shared::physics::air_step` wrapper (KH1's state-2 step, Steam `0x1402AF6A0`, through the state table) now calls `knockback::air_step` for enemies a KH2 blow launched and replaces the step's displacement with KH2's `Blow`: one 30 Hz tick every second frame, sideways ×0.8 floored at a tenth of the start, rise ×0.8, gravity 20/49, fall capped at 16, with `+0x3C0` / `+0x3C4` kept in step. KH1's mover, collision, landing, lying and get-up stay KH1's. The flight ends when the enemy leaves state 2, when KH1 writes another arc, or after 600 frames. Checked (decomp): KH2's velocity provider `0x140410330`. Assumption: the 30 Hz tick (docs/combat-model.md), not re-checked against a KH2 trace. docs/kh1-launch.md §4.

### 2026-10-25: Stage 602 (testing branch: KH2 blows through KH1's launch)
- **Every KH2 blow launches through KH1's launch.** The tester (kh2-knockback-on-enemies m1): "(b) but make it a toggle in the config menu / (a) / (a) / go ahead". KH2's reaction routine (Steam KH2 `0x1403D2FE0`, groups by `0x1403CE980`) gives a blow its starting speeds in `0x1403CEA00`: up = ATKP knock-back 1, sideways = knock-back 2, the defaults 24 / 40 only when both are 0 (Checked (decomp)). Every group-4 hit now takes KH1's finisher row and a kind-5 record (`kh1rows::throws` is the group alone); after KH1's launch (Steam `0x1402B8530`, existing hook) KH2's blow is flown to its apex for `+0x3B8` / `+0x3BC` and `+0x3C0` gets KH2's sideways speed. Gone (answer 3 (a)): the "holding blow" rule (`Strikes::hold`, `HOLD_*`, the zeroed record direction) and Explosion's override `FARTHER` (912 is KH2's 15 up / 30 sideways). A KH2 flinch on one of our rows whose data launches gets KH1's flinch (kind 3). Enemy Weight (answer 1 (b), `knockback.rs`, ini `enemy_weight`, default on): launches × KH1's launch speed for the enemy (20 + 15 × stats `+0xC4`, btltbl `*0x142D22D30` `+0x14` / `+0x16`, Checked (decomp)) over 35. Assumption: stats `+0xC4` = 1 is KH1's full weight; each enemy's value is logged once. Distances unscaled (answer 2 (a)). docs/kh1-launch.md §4.

### 2026-10-25: Stage 601 (testing branch: R2 Camera Hold)
- **The camera hold option on R2.** The tester (kh2-camera-fixes, after m15): "R2 should be the button for locking the camera". t498's L2 Camera Hold is now **R2 Camera Hold**: the Config > Camera row is renamed (`Row::R2Hold`, text `0x7F00_0121` / `0x7F00_0122`), ini key `r2_camera_hold` (an ini with the old `l2_camera_hold` keeps its choice), `camera::R2_HOLD`. R2 is read by the same `r2_held` as the t413 camera hold (Steam raw pad word `0x14298C9B4`, bit `0x200`, so the keyboard's R2 counts). With it on and R2 held, KH2's follow and lock-on cameras get no right-stick turn or zoom and no mouse motion, and R3 doesn't reset; R2 still stops the swing-behind (Auto) as in t413. L2 Stick Menu stays on L2. Defaults stay Off and the two options stay separate (the tester's (b) / (b)); L2 + mouse is not done ((c), later). With Frame Advance on, R2 steps frames instead. Assumption: the button is the tester's choice (KH2's run-time remap table can't be read). docs/kh2-pad.md.

### 2026-10-25: Stage 600 (testing branch: Config: KH2 Damage row)
- **KH2 Damage row.** The tester (m3, answer 3): "(a)". A Config > Combat row "KH2 Damage: Off / On" for ini `kh2_damage` (default On, live, `kh2damage::kh2_damage` / `set_kh2_damage`). On: KH2's power on KH1's curve, as before. Off: KH1's damage exactly; the curve isn't swapped and `combat::scale_record` no longer rewrites the damage after the hit (the old after-the-fact rewrite now runs only where the hook can't be made, Epic). `damage_scale` / `magic_scale` act only with it On; reactions, launches and criticals don't depend on it. The Combat list is 12 rows now, drawn smaller by `list_scale` (about 0.74). Checked: code paths. docs/kh1-kh2-attack-data.md §9.

### 2026-10-25: Stage 599 (testing branch: Config: Party Criticals)
- **Party Criticals.** The tester (m3, answer 2): "in the config menu make a separate option for party member criticals". A new Config > Combat row "Party Criticals: Off / On" (ini `party_criticals`, default Off, live) decides instead of Criticals for an attack whose owner (`+0x34`) is an actor on Sora's side (`+0x374` bits 5..6 equal to his) and isn't Sora: Donald, Goofy and the guests (`kh2damage::party_member`). The side bits are the ones KH1's damage function itself tests (Checked (decomp)). Assumption: every actor on that side except Sora counts (a summon's own attack would follow this row too); an attack whose owner is gone follows Criticals. docs/kh1-kh2-attack-data.md §8.

### 2026-10-25: Stage 598 (testing branch: Criticals Off for every attack)
- **Criticals Off applies to every attack.** The tester (kh2-attack-data m3, answer 2): "lets do (b)". KH1 rolls its critical in its damage function `0x1402BFA80` for every attack, whoever owns it: a random value below the attack's `+0x94` adds `+0x98` to the power and sets record bit 0x20; only the journal tally (`0x140291FF0`) is the player's, and object targets never roll. Chance and bonus are set when the attack object is made (`0x1402B3270`: weapon factor on `+0x94`, bonus on `+0x98`). Checked (decomp). `kh2damage::damage` now zeroes `+0x94` for the call on any attack while Criticals is Off (Sora's, the party's, enemies') and restores it after; On is KH1's roll exactly. Help text and ini comment updated. Not checked: which enemies' attack data can crit. docs/kh1-kh2-attack-data.md §8.

### 2026-10-25: Stage 597 (testing branch: Aerial Dodge: no release at Physics KH1)
- **Aerial Dodge: no early release at Physics KH1.** The tester (aerial-dodge-native m3): "(b)" (no release at Physics KH1: the dodge always rises fully), "go ahead". On the frame a dodge starts at Physics KH1, `shared::physics::kh1_held_jump` now clears Sora's `+0` flag `0x40000` (KH1's early-release cut still open) instead of setting t545's `0x60000`. KH1's button handler `0x1402A7B30` (Steam, from the control step `0x1402A6220`) runs its variable-jump cut (`0x1402AF8F0 (actor, 5.0)` from air time 6) only while that bit is on, so the dodge always rises fully, held or let go; the fall still swaps at the apex (`combomods::dodge_apex`). No other KH1 code reads that bit on the actor (the other `0xE0000` tests are on attack objects). Checked (decomp). Physics KH2 unchanged (KH2's own release, `0x1403CB680`). docs/kh1-aerial-dodge-native.md §8.

### 2026-10-25: Stage 596 (testing branch: Debug menu: Cloth Physics page)
- **Debug menu: Cloth Physics page.** The tester (m19 6 (a and b)): the field-run physics on / off and each part's stiffness, damping, weight and swing cap live, plus Reset to tuned; the settings id in the page title and every change logged ("cloth physics settings id N ..."); session only. docs/kh1-secondary-motion.md.

### 2026-10-25: Stage 595 (testing branch: Trace: Cloth category)
- **Trace: Cloth category.** The tester (m19 3), off by default (ini `trace_cloth = off`, its own group): the animation drawn as the panel shows it, the physics state (on, baked / live, steps, restarts, settings id, max swing, tips pushed out), Sora's place / yaw / speed / turn rate per 60 Hz frame, every physics bone's turn against its parent and place against the hips, and the collision bones (3, 4, 17, 18, 43, 44, 71). docs/kh1-secondary-motion.md "the trace's Cloth columns".

### 2026-10-25: Stage 594 (testing branch: Trace Recorder: Frames)
- **Trace Recorder: Frames.** The tester (m19 2): with Game Frames on, the Seconds row reads "Frames" and shows seconds × 60 (it still steps by a second; the ini keeps seconds). docs/kh1-frame-advance.md.

### 2026-10-25: Stage 593 (testing branch: Frame Advance info panel)
- **Frame Advance info panel.** The tester (m19 1 (a), 2 (a)): a top-right box while Frame Advance is on, apart from the debug menu (`fapanel.rs`, drawn from `debugmenu::draw_end`): KH2 motion, battle / field, KH1 slot (and ours), drawn baked / live / by KH2's controller / KH1's own, the frame within the animation ("field run 12 / 40"), loops, the trace frame, cloth settings id and steps. Assumption: KH1 wraps a looping motion's time at its length. docs/kh1-frame-advance.md "The info panel".

### 2026-10-25: Stage 592 (testing branch: Cloth physics live on KH2's field run)
- **Cloth physics live on KH2's field run.** The tester (Missing sleeves animation m19): "lets do "another option" ... I can see how it looks in game before a baking starts", "3. ... the chain that sora has on his left hip", "4. physics only on the move", "start with soras running animation without the keyblade". New `kh1_data::clothsim` (shared with `examples/cloth_sim.rs`, so a later offline bake gives the same result) moves the coat (72, 248–279), shorts panels (26–41, 52–67), belt chain (10–15) and belt front (5–9): rest = KH1's field run (slot 5) averaged, a damped spring on the carrier's acceleration, lengths kept, swing caps, t569's clearance. `clothlive` runs it only on our slot for KH2's field run: baked, on KH1's own drawing of the bake (`+0x1B4` read back, changed bones written, `pose::cloth_on_kh1`); live, on our pose. One step per 60 Hz tick (`takeover::TICKS`). Tuned against KH1's run (stiffness / damping / weight / cap): coat 0.02/0.30/0.25/40°, shorts 0.20/0.45/0.25/30°, belt chain 0.06/0.05/0.25/100°, belt front 0.06/0.45/0.50/60°; swing 8.5 / 3.9 / 38 / 18° against KH1's 8.6 / 3.7 / 36 / 18. Checked (files). Assumptions: stride phase not matched (KH1's IK legs not evaluated); swing caps ours. Test `local_clothsim`. docs/kh1-secondary-motion.md "Cloth physics".

### 2026-10-25: Stage 591 (testing branch: Finishing Plus on KH1's own finishers)
- **Finishing Plus on KH1's own finishers.** The tester (m22): "you need to make sure every kh1 finisher can go into other kh1/kh2 finishers". While KH1's own finisher (plain or ability) plays at the KH1 setting with a finisher left, `combomods::kh1_finisher_window` opens KH1's combo window (instance `+0x14` bit `0x400000`) at KH2's 155 / 183 Finishing Plus window's share of the motion (`+0x16C` / `+0x280`, the share KH1 computes in `0x14029FF80`, Checked (decomp)); the walk picks the next one, and after KH1's own normal finisher the next normal one is KH2's by its walk (`0x140402C50`, Checked (decomp)): 154 with a foe in its window else 155, air 184 / 183 (`Character::normal_finisher`, sim test `normal_finisher_by_kh2s_walk`). Assumption: the window placed by share (KH1's finishers have none of their own). Single-target order by how far each moves the foe (docs only): Gravity Break 0 (`0x1402B7DE0`), Zantetsuken up to 80 (flinch push `0x1402B7E60`), Blitz launch (`0x1402B8530`), Checked (decomp, files). docs/kh1-anim-slots.md.

### 2026-10-25: Stage 590 (testing branch: Locked on, group checks count every enemy)
- **Locked on, group checks count every enemy.** The tester (m22 answer 4): "(a)". KH2's selector `0x140402D20` runs its kind-2 pass `0x1404029F0` on every target the iterator `0x1403BDAC0` gives on an attackable actor; the lock plays no part, only kind 1 uses the primary target (Checked (decomp)). `Character::row_targets` (the Base / Aerial Base Combo KH1 walk, `row_in_range`, `ability_row`) no longer keeps only the locked foe; the primary target and aim stay the locked foe's. KH2-setting selector (`try_attack`, stage 207's lock filter) unchanged. Sim test assertions added.

### 2026-10-25: Stage 589 (testing branch: Zantetsuken back, its KH1 rule)
- **Zantetsuken back, its KH1 rule.** The tester (m22 answer 3): "bring back zantetsuken and make it b as well". No longer taken out of Sora's list (stage 153's rule and the reward-line hook `0x1401B6990` are gone); a copy taken out before is given back once, off (`kh1-kh2-removed.txt`). KH1's `0x1402A6EE0` takes 0.3 off one random number per press for each equipped finisher in the order Stun Impact, Gravity Break, Zantetsuken (rows 3 / 4 / 5); row 8 only if all fail (Checked (decomp)). The walk holds bit 29 back and rolls Zantetsuken itself on that shared roll, always (its own rule, whatever KH1 Ability Chance says), in the single-target group after Gravity Break, before Blitz. Row 5 data (btltbl `0x4FD8`): gravity, power 250, reaction 3 (Checked (files)). Not checked: where KH1 grants it. docs/abilities.md, docs/kh1-anim-slots.md.

### 2026-10-25: Stage 588 (testing branch: Hurricane Blast keeps KH1's rule)
- **Hurricane Blast keeps KH1's rule.** The tester (m22 answer 3): "(b)". KH1's air picker `0x1402A6C70` (Steam, Checked (decomp)) marks row 15 only in state 2 at the finisher, with a target whose lock point (`0x1402B04C0`) is no more than 100 above him (y down), its request not the one playing. Bit 25 is no longer held back (`kh1picker::HELD_AIR` 0); the walk takes Hurricane Blast in Aerial Finish's place only when KH1's picker returned 15 (`Kh1Rule::Kh1Test`, as Blitz); the plain finisher is then 16 (18 if 16 plays), Blitz's 8 (13 if 8 plays; it fell back to 6 before). docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 587 (testing branch: KH1 Ability Chance off by default)
- **KH1 Ability Chance off by default.** The tester (m22 answer 2): "(b)". `settings` default false, ini template `kh1_ability_chance = off`. An ini line already written keeps its value.

### 2026-10-25: Stage 586 (testing branch: Tweaks inside Combat)
- **Tweaks inside Combat.** The tester (KH2's combo row selection, m22 answer 1): "(b)". Tweaks is Combat's tenth row (after Magic), its list's parent Combat; KH1 in KH2 is back to Camera, Combat, Movement, Misc. A list that would reach below the help box (y 300, def `0x140546160`) is drawn with KH1's window scale (`+0x6C` bit 0, `+0xA0` / `+0xA4`, anchor top) at 300 / its bottom: Combat at 0.85, the glint re-aimed through it (`config::list_scale`, `glint_to`). Checked (docs/kh1-config-menu.md §5 layout). Assumption: 85% fits and reads well. docs/kh1-config-menu.md §6.y.

### 2026-10-25: Stage 585 (testing branch: KH2's moves on KH1's states are left only in KH2's free state)
- **KH2's moves on KH1's states are left only in KH2's free state.** Same ask (kh2-magic-and-reflect-native m9). KH2's swings on KH1's combo requests kept the KH1 template's own "may be left" point, where KH1's control step jumped or rolled him out. `kh2slots::Leave`: swings, our rows and casts get KH1's "may be left" event only from KH2's free state, so none; KH1's action flag clears at the motion's end (row runner `0x1402A68F0` and `combomods` queue with `0x40000000`, cleared by `0x14029F6E0`; Checked (decomp) KH1 Steam). Reactions keep KH2's free state or else the template's point (Assumption: some KH1 reaction idles are queued without the bit). `roll_free` uses range id 2 (36). `cast_cut_open` lets a jump or Square cut a KH2 cast on KH1's cast state only in a free state, never for Sora's; Attack still from the reaction-command window. docs/kh1-anim-slots.md "Leaving KH2's moves on KH1's states".

### 2026-10-25: Stage 584 (testing branch: Jump and Square leave a move only in KH2's free state (controller))
- **Jump and Square leave a move only in KH2's free state (controller).** The tester (kh2-magic-and-reflect-native, m9): "you can jump and dodge roll out of moves during its end frames (I think around the same frames as when you can attack?) so this needs to be looked into the steam c decomp so its exact." Steam KH2's input step `0x1403D6FC0` calls the jump handler only with `+0x18C & 0x1C` (range ids 0 / 1 / 2, set by `0x14040D810`) or in the landing (motion 5); the action button's guard, roll, Quick Run and glide test the same through `0x1403BFB30`; Attack (`0x1403D6CA0`) takes the combo window (0x400), the finisher window (`+0x194` 0x20) or a free state. Checked (decomp). Sora's swings, ability moves and casts have no free state; the landing's is 12, the roll's 36, the guard's 52, the flinches' 50 (Checked (files)). `Clip::free_at` / `first_free`; `Character::run_action` takes a jump or Square only there (landing jump at any frame); the roll is left from 36, not its fall check at 34; flinches no longer from frame 8. `Character::can_leave` replaces `can_take_command` in `takeover::kh2_jump_open`. docs/movement-model.md "Leaving a move".

### 2026-10-25: Stage 583 (testing branch: Reflect's voice by Battle Quotes)
- **Reflect's voice by Battle Quotes.** The tester (kh2-magic-and-reflect-native, m7 answer 3): "if battle quotes is kh2 then (a) for kh1 we keep the grunts but the finisher will say "defence!" its one of the lines he picks when casting aero". `reflect::cast_voice` (both cast paths): at KH2, KH2's cast start's lines (Steam KH2 `0x1403C5170`: MAGC `+0x34` voice 7 on a cast, an even pick of five clips; `+0x35` voice 25 "Protect!" on the finisher; Checked (decomp, files)), through KH1's voice channel, both loaded at start. At KH1, the swing grunt (group 4) on a cast, and on the finisher Sora's KH1 Aero clip 46 at priority 3 through KH1's clip play (Steam `0x140290930`). Aero's group 13 has clips 46 / 47 (Checked (files)); Assumption: 46 is "Defense!", logged with its clip. docs/kh1-reflect-spell.md, docs/kh1-voice.md.

### 2026-10-25: Stage 582 (testing branch: Reflect stops enemy shots like KH2)
- **Reflect stops enemy shots like KH2.** The tester (kh2-magic-and-reflect-native, m7 answer 2): "(a) it needs to work exactly as kh2"; m9: "if you need to add more features into kh1's engine to get reflect working then do it". KH2's meeting `0x1403D2140` marks a shot's attack over (flag `0x200`) and tells its script; the barrier stays and nothing is sent back (Checked (decomp, KH2)). On Steam, `combat::stop_shot` replaces KH1's clash half `0x1402B4BC0` (which reflected free shots): the shot gets KH1's struck-no-damage result 4 (`+4`, `*(+0x60)`, as `0x1402B4060`), its hit label runs as when it strikes an actor (`0x1402BCC00 (process +0xA0, label +0x38, 1, &Sora's handle, 0)`, as `0x1402BF330`), and it is switched off (activity `0xC` cleared, as `0x1402BBC10`) so KH1's attack pass `0x1402B47A0` frees it. Checked (decomp, exe). Team and owner unchanged, still counted for the counter, contact effect still plays, melee unchanged (t573), Epic unchanged. Assumption: each enemy's hit label is its own reaction (not read per enemy). Logs "Reflect barrier stopped enemy shot ...", "... still flying 10 frames after meeting the barrier". docs/kh1-reflect-spell.md.

### 2026-10-25: Stage 581 (testing branch: Config: Tweaks list, KH1 Ability Chance)
- **A Tweaks list with KH1 Ability Chance.** The tester (combo row selection m21): "lets make a toggle in the config for the 30 percent chance to exist, if it does then if the 30 percent chance roll fails then it picks a similar kh2 move. these will be in the gameplay config in a sub menu called Tweaks". KH1 in KH2 gets a fifth button, Tweaks (`config::List::Tweaks`, the same KH1 window definition copy and back-out as the other lists). Its row KH1 Ability Chance (Off | On) switches `kh1picker::chance` live, saved as `kh1_ability_chance` (default on). Assumption: "gameplay config" = the KH1 in KH2 list. docs/kh1-config-menu.md §6.y.

### 2026-10-25: Stage 580 (testing branch: Combo: one finisher walk in KH1's picker (KH1 rule set))
- **KH1's and KH2's ability finishers in one walk in KH1's picker.** The tester (combo row selection m21): "do 1 ... you need to keep blitz's rule", "gravity break and blitz last, if its blitz and gravity equipped then blitz last"; m20: "(a) like kh2", "for explosion it should come last, for hurricane blast it should come first". `kh1picker` holds back Sora's ability bits 26 / 27 / 28 (ground) and 25 (air) (Steam `0x142D5EC10`, set by `0x1402A6380`, Checked (decomp)) while KH1's pickers `0x1402A6EE0` / `0x1402A6C70` run; KH1 still decides its count, length, dash, drop, Zantetsuken and Blitz (its own test, row 11's count, `0x1402A7320`, returns row 6). At the finisher `combomods::finisher_walk` goes through KH2's PTYA order (Checked (files); KH2 `0x140402C50` takes the first passing row, no chance, Checked (decomp)), each group gated by its KH2 row's range checks (`Character::row_in_range`): ground 26 [Stun Impact, Ripple Drive, Explosion]; 2 [KH1's finisher]; 27 [Guard Break, Gravity Break, Blitz]; 42 [Stun Impact, Ripple Drive, Explosion]; KH1's finisher; air 49 [Magnet Burst]; 7 [KH1's finisher]; 29 [Hurricane Blast, Aerial Finish]; 50 [Magnet Burst]; KH1's finisher. KH1's moves need their bit and KH1's anti-repeat; Stun Impact / Gravity Break take KH1's 30% roll with `kh1_ability_chance` on (KH1's random number `0x14031BA30`, prologue Checked (exe)), a failed roll moving on; none of KH1's moves when its picker took its dash (row 1). KH2's moves use `Character::ability_row_id`. Assumptions: KH1's finisher in the 154 / 184 place; Zantetsuken left to KH1; Hurricane Blast on Aerial Finish's checks. Logged ("KH1's ground/aerial finisher walk"). docs/kh1-anim-slots.md §7.z.1.

### 2026-10-25: Stage 579 (testing branch: Effects: Fire's hit area at KH1 Sora's size)
- **Fire's hit area at KH1 Sora's size.** The tester (m20, answer 3): "(a)". Fire's KH2 attack shapes (base Sora's KH2 Fire and Wisdom's) go through Reflect's barrier scaling, renamed `reflect::kh1_sized_shapes`: offsets from his place, radius and height x `pose::kh1_size` (0.89), matching Fire's effects since t561. Checked (code): the same path as t518. Blizzard, Thunder and Cure keep KH2's size. `docs/kh2-effects.md` §18.22.

### 2026-10-25: Stage 578 (testing branch: Magic: Air Physics KH2 holds KH2's hover on KH1's air cast animation)
- **Air Physics KH2 hovers him like KH2 on KH1's air cast animation.** The tester (m20, answer 2): "(b) hold him like kh2, the player expects it to be like kh2". `shared::physics::kh1_cast_hold`: with Air Physics KH2, while KH1's own air cast animation plays, `air_step` holds him in the no-gravity range of KH2's air cast clip for that spell (`kh2slots::CASTS`: Fire 58 16..74, Blizzard 61 16..66, Thunder 64 / Cure 67 16..58; Gravity on Thunder's, Stop and Aero on Cure's; Checked (files) P_EX100.mset), timed in 60 Hz frames from when KH1's air cast request began, KH2's arc clock running through it. KH1's air cast motions (slots 80 / 81: 78 / 68 frames, Checked (files)) outlast the holds. Air Physics KH1 unchanged. `docs/kh1-magic-native.md` §4i.

### 2026-10-25: Stage 577 (testing branch: Magic: KH1 spells on KH2's cast animation cast on its cast frame)
- **KH1 spells on KH2's cast animation come out on KH2's cast frame.** Same ask ("and vice versa"). KH1's script-call table entry 5 (`0x140529180 + 0x50`; original `0x1402B98E0`, Checked (exe); read through `0x140529FA0`) points at `formmagic::script_step`, which calls KH1's and, only for that cast's script process (the task's `+0x38` from the spell start `0x14028F140` / `0x14028F5E0`, matched with the running process `0x142DB7B48`, Checked (decomp)), scales the returned frame by KH1's cast frame over KH2's (the clip's magic-cast event) until KH2's cast frame. KH1's wait then ends on KH2's frame; its check, shot, MP and effects stay KH1's (ground Blizzard 28 on 16, Thunder 30 / Gravity 28 on 18, Cure 36 / Stop 28 / Aero 30 on 18; air 28 on 36, 30 on 30). Steam only. Assumption: +-1 frame. `docs/kh1-magic-native.md`.

### 2026-10-25: Stage 576 (testing branch: Magic: KH2 spells on KH1's cast animation, released at KH1's cast frame)
- **KH2 spells on KH1's cast animation come out on KH1's cast frame.** The tester (kh2-effects-in-kh1s-own-effect-system m20, answer 1): "if a kh2 spell is casted on a kh1 animation then it should cast on the frame that the kh1 animation casts it and vice versa". `formmagic::anim_kh2` follows each spell's Animations row for KH2's Blizzard, Thunder and Cure too (Fire has no row). At KH1, `kh2slots::redirect` leaves KH1's cast slot on the request and `kh1_spell_start` starts KH2's script; `shots_step` holds the script Sora in KH2's cast motion until KH1's frames since the start (frame scale `0x14233FBDC` per physics frame) reach `KH1_CAST_FRAME` (Blizzard 28, Thunder 30, Cure 36), or KH1's applied cast request `+0x164` / action flag goes. Checked (files): KH1's `xm_*0N.mag` scripts, all levels, read through KH1's script machine `0x1402CC620` (Checked (decomp)) and call table `0x140529180` (Checked (exe)). Assumption: +-1 frame from task order. KH2's cast motion's own frame events don't play on KH1's animation. Config help reworded. `docs/kh1-magic-native.md` §4j, `docs/kh1-move-styles.md`.

### 2026-10-25: Stage 575 (testing branch: Glide: bank changes logged (KH2's restart rule checked))
- **Glide bank changes: KH2's restart rule checked, each change logged.** The tester (m6): "when sora turns to another bank after hes banking in a direction he resets his animation which doesn't seem like kh2." KH2's glide update `0x140423A70` asks `0x1403C6DC0(holder, 205/206/207, 16, 0)` every frame; a motion not already playing starts at time 0 with a 16-frame blend, play time carried only between walk and run (Checked (decomp)). 205..207 are 30-frame loops (A170..A172, Checked (files)). KH1's request `0x14029F7C0` / apply `0x1402A0350` do the same, with a mid-blend path `0x140393B30` from the `+0x1B0` channels (Checked (decomp)). So the restart is KH2's rule; no change to the animation. Each glide motion change is logged (first 80) with KH1's slot, time and blend. Not checked: KH2's blender mid-blend (`0x1403C88C0` / `0x1403C8CD0`). `docs/kh1-glide-native.md` §8.9.

### 2026-10-25: Stage 574 (testing branch: Glide: the ability list follows the Glide style at once)
- **The Glide style swaps the ability list the moment it's changed.** The tester (KH2 Glide thread, m6): "glide and super glide don't get removed anymore when setting glide to kh2, and kh2's glide growth ability isn't there." KH2's own Glide (id 88) replacing KH1's Glide and Superglide in the shared list (`growth::glide_swap`, t323) only ran once a second from `menu::Growth::update`, which only Sora's physics frames call, so after switching the style in the paused Config menu KH1's Abilities screen still showed KH1's two. Now `config::set` for the Glide row calls `menu::glide_style_now` at once (the swap, KH2 Glide's name with its level, a log line "abilities: Glide style ... set in the Config menu"); KH1 rebuilds Sora's ability bits `0x142D5EC10` on the menu's close (`0x1402A6380`). Checked (code); Checked (logs): the swap itself works in t544 and t567. Assumption: this timing is what the tester saw. `docs/kh1-glide-native.md` §8.8.

### 2026-10-25: Stage 573 (testing branch: Reflect: hit outside the barrier; melee meets it as KH2's guard)
- **Reflect: untouchable only while a barrier stands; melee clashes with it.** Same ask (m6 answer 3). KH2's 71 / 72 have no invulnerable range (range id 27, Checked (files)); the same is taken for 73 (Assumption from §4e). Sora's untouchable flag 0x40 is set only while a barrier stands, and `hurt` no longer evades hits through the cast (undoes main's s179 / s226). An enemy melee attack meeting the barrier gets KH1's clash with a guard, so its owner recoils, as KH2 stops the attack and plays the owner's clash reaction (`0x1403D2140`, Checked (decomp)). `docs/kh1-reflect-spell.md`.

### 2026-10-25: Stage 572 (testing branch: Reflect: KH2's script as it is (counter re-hits, burst, every cast its own))
- **Reflect's counter and burst as KH2's script runs them.** The tester (m6 answer 3): "reflect is currently not 1:1 with kh2, from how it falls to how it behaves in game, please look at the steam c decomp". Checked (files): ATKP 573 kind 3 (special guard), power 0; REFLECT_1.mag through kh2-magic: barrier from script frame 2 for 27 frames; no hits, no counter; with hits 574 / 576 / 578 (power 50, flinch, re-armed every 10 / 8 / 6 frames) 30..57, then the burst 667 / 668 / 669 58..63. Checked (decomp, KH2): the barrier meeting `0x1403D2140` counts on the barrier's own attack `+0xB0`. Every script attack strikes with its own row (undoes main's s229 / s233 single pop); a second Reflect leaves the first running with its own barrier, count and counter (undoes s175 / s182; Assumption: KH2 never cancels a running Reflect); hurt's blocks go to the newest barrier; the burst sound (element 7) on 667..669's first frame. `docs/kh1-reflect-spell.md`.

### 2026-10-25: Stage 571 (testing branch: Reflect in KH1's flight cast)
- **Reflect in Neverland flight on KH1's own flight cast.** The tester (m6 answer 2 (a)). KH1's flight cast `0x1402A4150` (Checked (decomp)): usable check; with a target in the command record (`+4` handle, `+8` lock point) aims the move direction at it (`0x1402B04C0`, `0x140386D10`) and sets stats `+0xD0` to pi/16; the air request `| 0x1000000` with KH1's flight motion queued (`0x40000075`, blend 8.0); the spell start; no hop, no halving. `reflect::native_ok` takes state 8 and `cast_native` does those steps with our air request (motion 73). Drive Forms not moved (a form still runs on KH2's controller, no KH1 cast state under him). `docs/kh1-magic-native.md` §4f.

### 2026-10-25: Stage 570 (testing branch: MP: KH2's cost kinds in KH1's engine (Cure: all MP, one bar))
- **KH2's MP cost kinds in KH1's own MP check, payment and menu.** The tester (kh2-magic-and-reflect-native m6, answer 1): "(a) based off of the config; we need to give kh1's menu and mp the ability to allow costs like kh2 cure ... it wipes your whole mp bar. this needs to be a feature in the engine ... at least 1 bar of blue mp before you can cast kh2 cure". KH1's MP check `0x1402A4C10` and payment `0x1402A5B60` (hooked) take two new kinds beside KH1's (units under 100, bars x 100): `0x40000000 | 1` all MP with at least one whole bar (KH2 Cure, the tester's bar rule), `0x20000000 | percent` a share of max MP. KH1's usable check `0x14028EFD0` is hooked: for base Sora's Fire..Cure with their Magic row at KH2, the spell data's `+0x30` reads the KH2 kind for that call only, so the Magic menu fill `0x1402944C0`, the shortcuts and the command step grey and refuse by KH2's cost (KH1's at the KH1 row). Our KH2 casts pay through KH1's payment. Checked (decomp): KH1's check, payment, usable check and callers; KH2's `0x140401D60` / `0x1403D88E0` / payment `0x1403D3CF0`. Checked (exe): the three prologues. Difference by the tester's choice: KH2 lets Cure (and short spells) cast on any MP. Assumption: the per-command flag `0x1403C5940` / `0x1409ACFF4` unidentified. `docs/kh1-magic-native.md` §6, `docs/kh1-kh2-mp.md`.

### 2026-10-25: Stage 569 (testing branch: Hood, coat and shorts from KH2's hidden keys (test bake: walk and run))
- **Hood, coat and shorts from KH2's hidden keys, test bake in the walk and run.** Mr Bugs (Missing sleeves animation), after t552: "sora's hoodie still stays rigid ... it needs to look natural and his clothes not clip"; he picked option (c): "no it should be like kh2" (no lag or settling), "(a) just do a test bake into soras running and walking animations for now". New `kh1_data::cloth::ClothBake` runs in `pose::Baker::poses` only for KH2's walk (1) and run (2), battle and field (`CLOTH_MOTIONS`), after the retarget and `Secondary`, over the whole motion: hood 80-83 from KH2's hood keys 72 / 73 against the chest 68 (as at t375, without `KH2_SHOWN_ON`); coat from `coat_kh2` as now; shorts 27/31/35/39 and 53/57/61/65 sway as KH2's trouser leaves on that thigh sway about their mean, blended by place round the thigh. Ours, not KH2's: centred Gaussian smoothing (sigma 1.2 frames, wraps the loop, no lag) and clearance (hood 7 off the spine, 10 off the head; coat outside the shorts, at most 5 nearer the hips' axis than in the bind; shorts at 0.95 of their bind distance), cleared by a swing about one axis fixed per motion (no flips), spread over +-3 frames. Live poses and other moves unchanged; one log line per bake. Checked (files): KH2's keys and binds, all four motions (`examples/cloth_check.rs`, test `local_cloth`). Assumption: which trouser bone stands for which KH1 panel; the shorts' radius (from bones, not the mesh). `docs/kh1-secondary-motion.md`.

### 2026-10-25: Stage 568 (testing branch: Battle quotes: KH2's line cut through KH1's voice channel)
- **KH2's rule for Sora's battle quotes: a new line at least as important cuts the one playing.** The tester (KH2 sounds thread, m5 question 1: "(b)", then "do your recommendation, go ahead"). KH2: the quote request `0x1403CE580` takes a line when nothing is pending or playing, or when its priority is at least the last one taken; its voice step `0x1403CE260` stops the playing handle and starts the new line (Checked (decomp, KH2)). KH1's voice channel keeps no priority (`0x140290930` / `0x140290080` compare only a standing; Checked (decomp)), and `0x1400DE010 (2)` pauses effect and voice records rather than stopping them (same call as mode 1, `0x140056860` / `0x140056890`; that it is a pause is an Assumption from the pairing). KH1 ends a record with the engine stop `0x1400565A0 (sound, &0)`, as `SslSeStop` `0x1400DEF50` and the frame clean-up `0x1400DC8C0` do (Checked (exe, decomp)). Now `kh1bank::voice`, when Sora's channel (`0x142D371D0`) holds one of our KH2 quotes: a lower line is dropped; otherwise our bank's state-3 records in the live list `0x1404D6618` get the engine stop at fade 0, the channel goes to state 0 and KH1's own clip request takes the new line; KH1's clean-up deletes the stopped records. KH1's own lines keep KH1's rule. Logged "KH2 quotes: clip ... cut for clip ..." / "... dropped". Assumption until heard: the stopped record ends at once. `docs/kh1-sound-native.md`.

### 2026-10-25: Stage 567 (testing branch: Battle quotes: KH1's swing voice held back where KH2's quote plays)
- **KH1's swing grunt no longer takes Sora's voice channel ahead of KH2's quote.**
- Found in the tester's t544 logs (Battle Quotes KH2): every KH2 quote asked of KH1 was refused ("KH1 kept the line playing", all of them).
- The cause:
  - KH1's attack row start `0x1402A3980` ends with the row's `+0xF` voice through the voice group `0x1402901F0`. The call is at `0x1402A3AAF` (return `0x1402A3AB4`). Checked (exe).
  - Sora's rows carry one: air 16 / 17 / 18 have 5 / 4 / 4, ground 13 / 14 have 4 (`btltbl.bin` `+0xA9F0`). Checked (files).
  - The group then holds the channel for Sora (standing 2). KH1's clip request `0x140290930` only asks when the standing is below the new priority, so KH2's priority-1 / 2 quote never got in. Checked (decomp).
- Now, with Battle Quotes KH2 and KH2's voices loaded, `soundlog`'s voice-group hook leaves that call out on Sora when the row's request plays a KH2 motion of ours.
  - The call is told by its return address.
  - The row is read from combat state `+0x40`, which the row start writes just before. Checked (decomp).
  - The request comes from Sora's row table; `kh2slots::plays_kh2` = KH2's row selection picked a motion for it.
  - The KH2 motion's own quote takes its place, as in KH2, where a swing has only its own line.
- KH1's other voices of Sora, and rows that play KH1's own motions, are unchanged. Logged ("KH2 quotes: KH1's swing voice of row ...", first 20).
- Open: KH2's own priority rule. KH2 `0x1403CE580` / `0x1403CE260` take a line of priority at least the last taken and stop the playing one (Checked (decomp)). KH1's voice path never stops a line, so while Sora's last KH2 line plays the next is dropped. Asked in the thread.
- `docs/kh1-sound-native.md` "KH1's swing voice and KH2's quotes".

### 2026-10-25: Stage 566 (testing branch: KH2 sounds: one copy of each bank (RAM))
- **One copy of each KH2 sound bank.** This answers the tester's RAM question from m4 ("are we caching a bunch of sounds?").
- KH1's bank-entry destructors (`0x1400DB470` / `0x1400DB7A0`) free the entry's `+0x20` with `_aligned_free` only when it isn't 0. No KH1 function that walks the bank list `0x1404D65D8` reads `+0x20` / `+0x28` afterwards. The engine keeps its own pointer (bank `+8`, `0x14005E0B0`). Checked (decomp).
- So `kh1bank` now:
  - makes each bank's buffer once, with KH1's `_aligned_malloc`;
  - drops our `Vec` at that point;
  - sets the entry's `+0x20` to 0 right after KH1 takes it;
  - registers the same buffer again after each KH1 bank clear.
- KH1's engine writes into a bank as it loads it: ActualBank set-up `0x1400748F0` (vtable `0x140428790` entry 2) sets byte `+0xF` of each sound record to 0xFF. Checked (decomp, exe).
  - `scd_cut::as_loaded` writes the same up front, and again before each re-registration.
  - The buffer is then checked against the hash taken at the cut. A changed bank is logged and left out.
- Assumption: nothing else in an ADPCM bank changes during play. The log will say.
- RAM: ~9.0 MB (one copy) plus the voice bank's 0.6 MB. Before this round it was ~77 MB; before t509, ~58 MB.
- The voice bank keeps KH1's copying registration (`0x1400E0110`).
- `docs/kh1-sound-native.md` "One copy of each bank". Test `as_loaded_marks_each_sound_record`.

### 2026-10-25: Stage 565 (testing branch: KH2 sounds: only through KH1's engine (the old player removed))
- **KH2's sounds go only through KH1's own sound engine; the mod's player and its decoded audio are gone.** The tester asked: "I want you to gut the old system and transfer everything ... get it fully working."
- `kh2sound.rs` no longer decodes anything. The loader reads each KH2 bank, cuts the wanted sounds into one-sound banks (`scd_cut::cut`), hands them to `kh1bank` and frees the whole banks. That removes ~56 MB of decoded effects plus the decoded quotes.
- The battle-quote clip list keeps the clips whose record has audio.
- Removed:
  - the `waveOut` player (`play` / `tick`, the PCM copies, `sys.rs`'s winmm imports);
  - its volume read (`game_volume`, KH1's settings block);
  - the ini's `kh2_sound_engine` and every "mod's player" fallback.
- A sound KH1's engine can't take at that moment isn't heard, as KH1 drops its own. That covers a bank not yet back after a room load, the queue full, or a voice line KH1's priority rule keeps. The first 20 are logged ("KH2 sound not played: ...").
- Our effect slot in KH1's queue (`0x140179AC0`) is still stopped with KH1's `0x140179D90` once its length has played and `0x1401789F0` says it's done.
  - The length now comes from the audio header (`scd_cut::length_ms`: Microsoft ADPCM blocks x samples per block).
  - A stream with no length in its header is stopped once KH1 has said it plays and then that it doesn't (2 s if it never starts, 30 s at most).
- Off Steam nothing is registered, so no KH2 sound is heard there.
- Checked (code).
- Checked (files): every effect stream in se999 / se000 / se500 is codec 12 (ADPCM); the voice bank is 31 Vorbis streams. The lengths match the decoder's own block math.
- `docs/kh1-sound-native.md` "Only KH1's engine". Test `adpcm_length_from_the_header`.

### 2026-10-25: Stage 564 (testing branch: Magic: Air Physics row)
- **Air Physics: KH1 | KH2 for every air cast.** The tester (same thread, m19: "change
air magic to air physics kh1 | kh2 so now you can cast kh2 fire with kh1 air physics like how we
did for reflect. remove anything related to the main physics setting that has magic in it and put
it in this new air setting"). Config > Combat > Magic > Air Physics (ini `magic_air_physics`, its
default the old `physics` value), `shared::physics::cast_rule`. Moved off the Physics row:
`cast_stops` (KH2's 40 hop and kept speed, or KH1's own 30 hop and half speed), `air_step` while a
cast plays in KH1's air state (new `casting_in_air`: action flag + KH1's air cast request 0x83..0x8A
at `+0x164`, or a cast slot of ours; KH2's arc / gravity / drift / no-gravity hold with the clock,
or KH1's step), and the KH2 controller's borrowed air casts (`takeover`'s `air_magic_falls`). New:
KH1's own spells in the air take the row's start too (`formmagic::spell_start` calls `cast_stops`
for them). A cast at Air Physics KH1 under Physics KH2 drops our arc so the fall after it goes on
from KH1's fields. Physics row help text points to Air Physics. Checked (decomp) for the KH1 / KH2
air cast numbers (t534's §4g); Checked (files) the request range. Assumption: KH1's halving and 30
hop come after the spell start in `0x1402A3600` (what §4g already relies on). With a KH1 cast
animation (no KH2 slot) there is no KH2 hover at Air Physics KH2 (asked). `docs/kh1-magic-native.md`
§4i, `docs/kh1-move-styles.md`.

### 2026-10-25: Stage 563 (testing branch: Magic: KH1's Gravity, Stop and Aero on KH2 cast slots)
- **Gravity, Stop and Aero on KH2's cast animation, natively.** The same ask (their
animation rows at KH2). Before, KH2's controller borrowed Sora for them and KH2's cast motion was
drawn over KH1's cast on its clock (`takeover::command_started`, `pose::follow`,
`BASE_KH1_CAST`). Now `kh2slots::CASTS` adds our slots of KH2's Thunder cast (62, air 64) for
Gravity and Cure's (65, air 67) for Stop and Aero on KH1's own requests 0x3A / 0x3B / 0x3C and air
0x87 / 0x88 / 0x89 (KH1's request table: slots 55 / 56 / 56, air 80 / 81 / 81, Checked (files)
`xa_ex_0010.mset`), and `combat::command` lets KH1's command step cast them there with KH1's own
script, cost and effects (`kh1_on_kh2`, KH1 states 0 / 6 / 0x18 / 2 / 8). The borrow is left only for
a state KH1 casts nothing in. KH2's cast windows (cut / combo, `cast_follow`) apply on them. Six more
baked slots at load. `docs/kh1-move-styles.md`.

### 2026-10-25: Stage 562 (testing branch: Magic: a Config row per spell and its animation)
- **Magic's rows per spell.** The tester (same thread, m19: "each magic should have its
own settings of which magic you can choose, e.g fire: kh1 | kh2 and that would change what gets
casted, we'll remove ground magic ... you can also change the animation of magic with the exception
being reflect, and fire"). Config > KH1 in KH2 > Combat > Magic is now Fire, Blizzard, Thunder,
Cure (what is cast, ground and air alike; ini `magic_fire`, `magic_blizzard`, `magic_thunder`,
`magic_cure`) and Animations, its own list (Blizzard, Thunder, Cure, Gravity, Stop, Aero: the cast
animation of KH1's spell; ini `anim_blizzard` .. `anim_aero`), so both lists keep KH1's full-size
rows on screen (config-menu-rework answer 1 (a)). The Magic and Aerial Magic rows are gone; the old
`magic`, `cure_style`, `kh1_cast_*` keys are read only as the new keys' defaults (an ini from before
casts what it did). `formmagic::kh2_spell` / `base_kh2_spell` / `anim_kh2` replace `base_kh2`; a
spell cast KH2's way always plays KH2's cast motion (its script needs it, §4 of
docs/kh1-magic-native.md); Fire's animation is its spell's, Reflect's KH2's. KH1's Blizzard, Thunder
and Cure with their animation row at KH2: KH1's requests 0x37..0x39 / 0x84..0x86 point at our KH2
cast slots (`kh2slots::redirect` by the new `cast_spell`), KH1's own script runs on them (no
borrow). Gravity / Stop / Aero at KH2 as before in this version (borrowed, drawn on KH2's motion).
The debug menu's KH1 Spells page now shows and sets the Gravity / Stop / Aero animation rows. Trace
CSV: the config field is 32 bits, columns `cfg_fire` .. `cfg_anim_aero` added (`cfg_magic` /
`cfg_air_magic` are the old keys' values, no longer used). Checked: the code paths (combat::command,
kh1_spell_start, redirect). Assumption: KH1's spell scripts time their shots by their own frames,
not by the cast motion (KH1's cast motions carry no spell events, Checked (files)), so KH1's spell
comes out at KH1's time on KH2's animation. `docs/kh1-move-styles.md` Magic.

### 2026-10-25: Stage 561 (testing branch: Effects: Fire at KH1 Sora's size)
- **Fire's KH2 effects at KH1 Sora's size.** The tester (kh2-effects-in-kh1s-own-effect-system
m19: "1.for fire do (a)", answering m18's question 1, "(a) scale every KH2 spell and move effect on
Sora to KH1 Sora's size the same way"). `kh2dpx::kh1_sized`, which scaled Reflect's packages since
t483, now also takes Fire's three packages (`formmagic::spell_package(0, level)`, base Sora's KH2
Fire and Wisdom's): each element's scale (KH1's instance scale, which carries its header offset:
the ring, its flame trails and the Keyblade pieces, places and sizes) and its position-only offset
x `pose::kh1_size` (0.89, the two bind poses' hip ratio). Fire's attack shapes stay KH2's size;
Blizzard, Thunder, Cure unchanged. Checked (decomp): KH1's instance scale `0x14031CEA0` (t483's
check). Assumption: none of Fire's pieces is placed by its script (`spell_place`, not scaled).
`docs/kh2-effects.md` §18.22.

### 2026-10-25: Stage 560 (testing branch: Hit effects: KH2 swings call the Kingdom Key's KH2 hit spark)
- **KH2 swings' Keyblade hits show KH2's own hit element, through KH1's lookup.** The tester (m2 answer 1: "when you hit an enemy an effect spawns on the enemy (keyblade hit effect, fire hit effect etc.) Please look into it and find an implementation thats native, I give you the go ahead"). `combat::swing_spark`: a hit of a KH2 swing's right-Keyblade shapes (weapon 1) by base Sora with the Kingdom Key (0x51, save `+0x36`) lands with `Kh2Spark { Pkg::Weapon, effect_on_hit, random_effect }` (Kh2Spark now carries a `kh2dpx::Pkg`) through t521's path: attack `+8` = the Kingdom Key's KH2 package (`W_EX010.a.us`), data copy `+0x28` = effect_on_hit − 1; KH1's contact (`0x1402b4060` → `0x1402b4320` → `0x1401dffe0` → `0x1402ae650`) picks, places and turns it. `kh2dpx::WEAPON_ON` builds hit elements 1 / 2 / 3 (kinds 1, 1, 0x102; Checked (files)); pppKeBornRnd5 (183) joins `KH1_SAME` (KH1 `0x14033E3D0` / `0x14033E4B0` / `0x140349EA0` vs KH2 `0x1404D4EA0` / `0x1404D4F80` / `0x1404E1780` decompile the same but the paused offset; constructs `0x14033E490` / `0x1404D4F60` same bytes; Checked (decomp), Checked (exe)). ini `kh2_hit_sparks` (default on). KH1's own swings, other Keyblades (answer 1 (c)), body shapes, Drive Forms and the hit sound unchanged. KH2's damage_effect is the target's own set (`target + 0x80`, kind 0x200, Checked (decomp) `0x1403d1b40`): KH1's enemies have none, left out. Assumption: a swing's attack carries its weapon's PAX. docs/kh1-kh2-hit-effects.md §9.

### 2026-10-25: Stage 559 (testing branch: KH2 effects: KH2's 0x4009 point on KH1's object root (Cure))
- **KH2's 0x4009 point is KH2's own: the object root plus the kind-9 shape's centre, at KH1 Sora's size.** The tester (kh2-hit-effects-on-enemies: "reflect, it still seems off centred, before these changes it wasn't how were you attaching it to sora? can we do the same while keeping it native? same goes for cure as well"; effects thread: "kh2 cure doesn't seem to be positioned properly either"). Since t470 `kh2dpx` started 0x4009 on KH1's body centre (0x8005) plus 20: KH1's first kind-1 collision part, a point along one of his bones (`0x14029EB20` → `0x1402B6D30`), which sways with the cast; the t544 log has it 90 up and 10 to one side ("body centre: KH1's 90.0 above Sora's place (offset [10.0, -90.0, -0.8])", Checked (trace)). KH2's 0x4009 (`0x1403B50F0`, Checked (decomp)) is the kind-9 shape's centre on the object root (Sora's 105 up, Checked (files) `P_EX100.mdlx`), which no motion moves. Now: KH1's object root 0x8000 (its frame §18.23) with the element's offset moved by that point (`effect_point_of`), and `kh1_point` scales a position-only element's offset on it by `pose::kh1_size` (~0.89, as Reflect's): Cure's elements 3 and 4 (`CURE_n.mag`, flags 0x245, Checked (files)) at ~86 / 93 above his place, centred. Logged ("on KH2's 0x4009 point"). Reflect unchanged (all its pieces are on 0x4000; none on 0x4009). docs/kh2-effects.md §18.16.

### 2026-10-25: Stage 558 (testing branch: KH2-only combo abilities follow the combo settings)
- **Finishing Plus / Negative Combo no longer turn the combo to KH2's; at KH1 they act on KH1's own combo.** The tester (m19: "the ability should not ignore the combat settings"). `styles::kh2` is the setting alone (t488's override gone; the log line now says the ability follows the settings). Negative Combo: `kh1picker::shorten` lowers the stats' length byte (`+0xD4` / `+0xD5`) by its copies (at least 1, KH2's rule) only while KH1's picker runs, so KH1's own finisher test marks the finisher sooner; `combomods::kh1_combo` reads the same length (`kh1_length`). Finishing Plus: KH2's Finishing Plus window on a KH2 ability finisher playing on KH1's rows (`kh2slots::finisher_window`) opens KH1's combo window only while a finisher is left (1 + copies, `finisher_chain_open`); the counted press's finisher is KH1's picker's (a KH2 ability finisher by KH2's checks, via `next_finisher`, or KH1's own finisher row), on a row whose request isn't playing; none, no attack. KH2 setting unchanged. Checked (decomp) for the length byte's readers in the pickers; Assumption: nothing else reads the byte during the picker's call. Not applied to KH1's swimming combo (state 0x18) or the spell combo-finisher test (`formmagic`). docs/kh1-move-styles.md.

### 2026-10-25: Stage 557 (testing branch: KH1's row picker picks KH2's ability moves)
- **At Base / Aerial Base Combo KH1, KH2's ability moves come out of KH1's own row picker.** The tester (m19: "if an ability is equipped thats kh2s and your combat config is set to kh1 then you should be able to do kh1s moves, and its row picks, we need to modify kh1s so that it can choose explosion and other moves during its kh1 combo", "patch the extra functions that kh2 uses in kh1"). New `kh1picker.rs` wraps KH1's ground picker `0x1402A6EE0` and air picker `0x1402A6C70` (Steam only; prologues `4C 8B DC 55 56 57 49 8D AB 48 FF FF FF` / `40 53 55 56 57 41 54 41 56 41 57`, Checked (exe)). KH1's picker runs whole (its count, its length test at the stats' `+0xD4` / `+0xD5`, its own ability rows by `0x142D5EC10` and chances, its target, its anti-repeat; Checked (decomp)); where it took one of its plain rows (ground 1, 8, 11, 13, 14; air 16, 17, 18), `combomods::picker` adds KH2's ability rows as the next candidates: the first equipped ability row of the kind KH1's count is at, playable on KH1's rows, not the KH2 motion playing, passing KH2's own row checks (new `Character::ability_row`: KH2's `condition` and `in_range`, KH2 Steam `0x140402750` / `0x1404029F0`, Checked (decomp) in earlier stages). The picker returns the KH1 row it runs on and the row runner plays it as before. KH2's selector (its table walk, its length) no longer runs for the Attack combo at the KH1 setting; t537's filter is gone. Flight (state 8), Drive Forms and the KH2 setting unchanged. Assumption: KH2's ability rows go after KH1's own ability rows and before its plain rows. docs/kh1-anim-slots.md §7.z, docs/kh1-move-styles.md.

### 2026-10-25: Stage 556 (testing branch: Recoil KH2: refact-self-2 swings make a guard's owner immune)
- **KH2's refact self 2 against a guard, on KH1's meeting.** The tester (m2 item 2: "add it and include in the kh2 config for recoil"). KH2 `0x1403D2140` (Checked (decomp)): a normal refact-2 attack meeting a guard goes on and the guard's owner becomes immune to it. KH1: the pass `0x1402B3BE0` calls the meeting `0x1402B4A20 (entry, entry)`, which runs the directional tests `0x1402B4640` and the halves; our refact-2 rows (0x62+) against a guard (100) make both halves do nothing, so the swing hit the body behind. The pass skips a body whose `+0x3E0` holds the attack's `+0x54` bit (cleared on re-arm `0x1402B3380` / free `0x1402B4460`). New hook `recoil::meet` (Steam `0x1402B4A20`, prologue `40 53 56 57 41 56 48 83 EC 78`, Checked (exe)): after KH1's meeting, at Recoil KH2, Sora's attack of a refact-2 row (`kh1rows::unstoppable`) against an enemy guard (priority 100), both directional tests passing (`0x1402B4640`, bytes checked before the call): the guard's owner gets the swing's hit-once bit, with KH1's clash spark (KH2's clash effect 0xD). Recoil KH1: nothing. Assumption: KH1's pass tests the meeting before that frame's body. docs/kh1-kh2-recoil.md §6.9.
- 1:1 check: KH2 decomp (§1.2), KH1 decomp of the pass / meeting / halves / re-arm, exe prologue.
- Not checked: in game; the pass's entry order (above).

### 2026-10-25: Stage 555 (testing branch: Recoil KH1: Guard Break goes through guards)
- **Guard Break pierces guards at Recoil KH1 too.** The tester (m2 answer 1: "guard break should function like how it does in kh2, it should pierce any normal enemies"). At KH1 our Guard Break row (row 69) gets priority at least 0x62 (`kh1rows::GUARD_THROUGH`; KH1 adds the Keyblade byte, cap 99) in place of the template's: KH1's half `0x1402B4BC0` meets two attacks only when one is under 0x62 and never stops a side at 0x62+ (Checked (decomp)), so a guard (100) never meets it and it reaches the body. An enemy attack under 0x62 still meets it and is stopped, its owner recoiling by KH1's rule (kind 7 past a gap of 40): KH1's, not KH2's (KH2's pierce meets no normal attack). Recoil KH2 unchanged (type 6). Counterguard and Finishing Leap keep their templates' priority at KH1. docs/kh1-kh2-recoil.md §6.8.
- 1:1 check: decomp of the half's priority test.
- Not checked: in game.

### 2026-10-25: Stage 554 (testing branch: Recoil KH1: no KH2 guard windows in Sora's attacks)
- **At Recoil KH1 Sora's attacks carry no KH2 guard.** The tester (m2: "guard break and other kh moves still use the kh2s recoil mechanics when I set recoil to kh1"; his trace `trace_1792887810_f39547` and the t544 log: Guard Break's own Keyblade guard, ATKP 162 frames 5..42, went up as KH1 guard objects and `hurt` took the enemy's three hits on it, "KH2's guard on the Keyblade (ATKP 162, refact self 2), no effect"). KH1's swings carry no guard: an enemy attack meeting one is KH1's clash by priority (`0x1402B4BC0`), or it hits him (Checked (decomp)). New `recoil::guard_window_on(motion, weapon)`: at Recoil KH1, base Sora's Keyblade guard windows (KH2 weapon 1 / 2) guard only in the guard press 173 and its repel 42; Guard Break 165's and Vicinity Break 170's 162s don't (`combat::guard_step` puts no guard object, `kh2slots::keyblade_guard` gives `hurt` none). Body guards (weapon 0: Aerial Dodge, Flash Step, Aerial Recovery, Counterguard, Vicinity Break's 406) and Drive Forms unchanged; Recoil KH2 as before. Checked (trace), Checked (decomp). docs/kh1-kh2-recoil.md §6.7.
- 1:1 check: trace/log; decomp of KH1's half.
- Not checked: in game; whether the enemy then recoils against Guard Break (it should: V3's priority).
- Overlap: body guards are the bodyguard helper's (left alone on purpose).

### 2026-10-25: Stage 553 (testing branch: Recoil: the Guard Repel row merged into it)
- **One Recoil row for recoil, parry and the guard repel.** The tester (kh2-recoil-and-parry-system m2: "we need to also combine guard repel and recoil into the same config"). The Movement list's Guard Repel row (t536, ini `guard_repel`) is gone; `combomods::repel_kh2` is the Recoil row (`recoil::kh2`): what base Sora's KH2 guard does on a block is KH1's own block (`kh1_block`: 0x6E / 0x6F by the priority gap, 8 frames, KH1's Counterattack) at Recoil KH1 and KH2's repel (motion 42, 200 back, 80 frames, Counterguard) at KH2. `guard_repel` no longer read or written (its default was kh2; `recoil` defaults to KH1, so a fresh ini and the tester's now get KH1's block). Recoil row help reworded. Checked (code). docs/kh1-kh2-recoil.md §6.6.
- 1:1 check: code paths only (repel_kh2's two callers).
- Not checked: in game.

### 2026-10-25: Stage 552 (testing branch: Sleeves thread: the coat turns as KH2's coat ring)
- **The coat moves as KH2's coat ring.** The tester (missing-sleeves-animation, after t519): "when I mean by sticking out is that the coat is stiff and not animated at all. redo the coat so that its actually animated ... this is happening on all kh2 animations that we've baked so far" ("we'll fix the clipping later"). t491-t519 kept the coat rigid on the chest (t519: hem on the hips), so under KH2's moves it never moved, live or baked. KH2's coat ring (root / leaf pairs under the chest 68: 69/70 back, 128/129 .. 134/135 +x, 220/221 .. 226/227 -x; each leaf's bind axis within 7 degrees of KH1's matching panel) is keyed by F-curves only: at its bind relation in the field idle, hanging down from the leaning chest in the battle stance and swings, swaying 9-21 degrees in the idle and run and up to 50-120 in swings, jumps and hit reactions (Checked (files): `P_EX100.mdlx` / `.mset`, all 83 motions; the earlier "held flared out" was that hang measured against the chest). `retarget::coat_kh2` (live and baked, replaces `coat_low`): each KH1 chain's joint (73, 249, 253, 257, 261, 265, 269, 273, 277) takes its leaf's turn from its bind with KH2's chest turn taken out, on KH1's chest 78 (as the hair takes its spike's); roots stay on the chest, the joints below keep their bind relation, lengths kept, no guard. Measured: a tip swings 33 degrees against the chest in a motion (median, max 109; was 0); out >2 off the hips' axis 44.5% (KH1's own coat 15%), inside the shorts 1.19% (was 0.55%), tips past 90 degrees from their bind 1.0% of chain-frames (jumps, spins, reactions); the run's +x front tip 8-12 into the shorts' front. Test `local_own` (`kh1_s_coat_turns_as_kh2_s_coat_ring_under_kh2_s_poses`). Tool `examples/coat_ring.rs`. docs/kh1-secondary-motion.md "The coat follows KH2's coat ring". Assumption: the `P_EX100` files on the owner's PC (`local/re`) are the Steam game's.

### 2026-10-25: Stage 551 (testing branch: Hit reactions: KH2's air flinch turn)
- **KH2's air flinch turn.** The tester (m2, question 2: "(a)"; question 3: "(b)"). KH2's air flinch (`0x1403D3840`) sets the push direction, negated for a hit from the front, as the facing at once; its ground flinch (`0x1403D43B0`) doesn't turn (Checked (decomp)). KH1's air flinch `0x14029CD80` doesn't turn; its knock-back `0x14029CEC0` turns by the same rule through `0x140297E10 (actor, atan2(x, z))` and `+0x150` (Checked (decomp)). `hitreact::air_flinch_turn`, with Hit Reactions KH2 after KH1's air flinch (`0x4E` front / `0x4F` behind, air state 2): KH1's facing set on the push direction KH1 just gave his air velocity (`0x14029D800`), negated for `0x4E`; no drift → no turn. Steam only (`0x297E10`, as glide.rs). Flight and swimming keep KH1's numbers (already the case: KH2's push only on `0x48..0x4B`, the air hop / turn only in state 2; Checked (decomp) 0x14029D110 uses KH1's push with `0x4E`/`0x4F`). Open: the flight flinch still plays KH2's 14 / 15 and Medium Hits applies there. docs §15, §16.

### 2026-10-25: Stage 550 (testing branch: Hit reactions: KH2's hurt line with Battle Quotes KH2)
- **KH2's hurt line with Battle Quotes KH2.** The tester (soras-hit-reactions-native m2, question 1: "(b)"). KH2's reaction motions say their line from frame trigger 14 at frame 0: flinches 10 / 12 / 14 voice 29, alternates 11 / 13 and the air flinch from behind 15 voice 30, blows 16 / 18 voice 1 (`P_EX100.mset`); `00battle` VTBL for Sora 29 clip 6, 30 clip 5, 1 clip 7, priority 3 (Checked (files)). KH1's hit pass `0x14029BBD0` ends with the voice group `0x1402901F0 (actor, group, 0)`: 0 flinch, 1 knock-back, 0x14 low HP, 2 HP gone, after the reaction is queued (Checked (decomp)). `hitreact::hurt_line`, from the existing voice-group hook (`soundlog::voice_group`) and only during Sora's pass on KH1's states, says KH2's voice for the queued request (`0x49` / `0x4B` / `0x4F` → 30, other flinches 29, knock-backs 1) through KH1's voice path (`kh2sound::hurt_voice` → `kh1bank::voice`) and skips KH1's; low-HP and HP-gone lines stay KH1's; not loaded → KH1's. In KH2's borrowed moves the hook's replay of KH1's group 0 / 1 is skipped with KH2's quotes on (the reaction clip's trigger 14 already says it). Follows the Battle Quotes row whatever Hit Reactions says (asked). docs/kh1-hit-reactions-native.md §14.

### 2026-10-25: Stage 549 (testing branch: Glide: the glide's speed kept after letting go)
- **The glide's velocity goes on into the fall when Square is let go.** The tester (same thread: "momentum currently doesn't work with letting go mid glide I just drop"). The t530 traces: 30.1/frame at the release, 1.1 the next frame, then +0.8 a frame. KH2's end (`0x1403CBC30` with 0, KH2 Steam) changes only the arc and the launch flag `+0xEC`, never `+0x20`: the whole velocity carries on (Checked (decomp)). Our end ran in KH1's control step inside KH1's think `0x1402A3300` (KH1 Steam), which then (flag `0x400` set by the state change, state 2, after state 2's pre-step `0x1402AFC20` copied the stick into `+0x20`) sets `+0x3C0` = `+0x20` × `+0x2C`, KH1's own carry into a fall from the stick (Checked (decomp)). `glide::carry` (from `takeover::physics`, before KH1's physics step, as `shared::physics::ground_hop_carry` does for the same carry) puts the glide's velocity back on `+0x3C0` on the frame after a release in the control step, so KH1's fall step (physics KH1) or KH2's arc on it (physics KH2) go on at the glide's speed. Ends in KH1's physics step (ground, end height) aren't affected (`0x400` cleared at the next think's start by `0x1402A43A0`; the Aerial Dodge apex, also in the physics step, kept its speed in the same traces, Checked (trace)). Logged "KH2's glide velocity ... kept into the fall". Also docs §8.7: every glide value checked against the traces (fmab Glide LV1..MAX, Checked (files)). docs/kh1-glide-native.md §8.4, §8.7.

### 2026-10-25: Stage 548 (testing branch: Glide: KH2's left bank (207) plays)
- **KH2's left bank 207 plays on KH1's request 0x74.** The tester (kh2-glide-in-kh1s-glide-state, after t526..t531: "sora can't bank to the left but can to the right"). KH1's glide pre-step (Steam `0x1402B2880`) asks 0x73 for a turn past +30° and 0x74 past −30°; KH2's pick (`0x140423A70`) was right (206 when `(facing × stick).y > 0`, else 207; `0x140142120` is a × b with a = facing, Checked (decomp)). But the set Sora is bound to in a world (88 slots of its own) has 0x74's entry on slot 77, the right bank's, where the file has 78 (`xa_ex_0010.mset`: 0x73 `0x204D`, 0x74 `0x204E`, Checked (files)): the t530 log's "not in this set: 0x74", and the trace drawing KH1's slot 77 (KH2's 206) for every left bank (Checked (trace)). So 207 was never bound and both sides showed 206. `kh2slots::also_stands_for`: KH2's 207 stands in for 0x74 when the set's entry holds slot 78 or 77. Assumption: KH1's and KH2's worlds turn the same way (both y-down), which decides which side is 206. docs/kh1-glide-native.md §8.3.

### 2026-10-25: Stage 547 (testing branch: Quick Run: out of KH1's roll at KH2's frame)
- **KH1's own roll turns into Quick Run at KH2's roll's frame.** The tester (quick-run-native m3: "with kh1 ... waits until dodge roll finishes to go into quick run ... doesn't get the same distance ... make sure it behaves the same as kh2 dodge roll", "go ahead"). The trace: KH1's roll `0xDC` from frame 9384, Quick Run at 9414 (30 frames, KH1's roll motion's end); with Dodge Roll KH2 the log has it 10 frames in. `shared::control` let a held roll into `quickrun::start` only on a frame KH1's can-act test passed (KH1 holds its action flag to the roll motion's end); KH2's roll on KH1's roll already bypassed it. Now KH1's own roll is cut the same way at KH2's Quick Run check (motion 209's trigger 34, frame 10 at 60 Hz, counted on the same counter from the roll's first frame). The Quick Run itself was already identical both ways (log: speed 30, brake 0.94, time 14, stop after 13 frames at 19.5). Checked (trace / log / files: motion 209 length 52, trigger 34 at 10, root 220.8 by frame 10). Assumption: cutting KH1's roll at its 10th frame (its 12-frame no-damage timer flags cleared by `quickrun::start`, as for KH2's roll) needs nothing else of KH1's roll ended. docs/kh1-quick-run-native.md "Out of KH1's own roll at KH2's frame".

### 2026-10-25: Stage 546 (testing branch: Body guards: KH1 guard objects on every KH2 body guard window)
- **Every KH2 guard window on his body is a KH1 guard object on KH1's states.** The tester (thread "Body guards on KH2 moves", from aerial-dodge-native m2 question 7: "(a) go and investigate those moves and ensure that they have the proper frame data and timings in its own thread", "go ahead"). `takeover::kh2_hits` took body `Guard` windows only for the Aerial Dodge (t494); now every `Guard` window of the KH2 motion KH1 plays, as KH2's controller does (no filter on its guard boxes), so `combat::guard_step` puts all-round priority-100 guard objects on his body: Aerial Recovery 40 (ATKP 124, frames 0..16), Explosion 166 (124, 32..64), Magnet Burst 197 (406, 20..70), Aerial Dodge 202 (406, all) as before. Enemy attack objects meeting one are stopped (KH1's clash on their side), no reaction on Sora (refact self 2). Frame data checked against the files: guard, invulnerable (range 27: AR 0..16, Explosion 32..64; none on Magnet Burst / Flash Step / Vicinity Break) all match what the code reads; KH2's range test `0x14040E740` is start <= t < end, Checked (decomp), the same as `Window::contains`. ATKP 124 kind 3 (special guard), 406 kind 2, both refact self 2, reflect range 0, Checked (files). Assumption: kind 2 and 3 act the same against KH1's enemy attacks (no KH2 kind on them). Flash Step 169 / Vicinity Break 170 aren't baked (Keyblade guard windows rejected by `blade_swing` / `body_strike`), Checked (trace: t544 logs, 170 borrowed); KH2's controller plays them with all their guards. docs/kh1-clash.md.

### 2026-10-25: Stage 545 (testing branch: Aerial Dodge: KH1's own held jump on it at physics KH1)
- **The Aerial Dodge at physics KH1 is held as KH1's jump.** The tester (aerial-dodge-native m2: "currently sora in kh1 mode cancels his air dodge, that doesn't fit with how kh2 does air dodge in the base game", traces f80604 / f80807, Physics KH1, Jump KH1). Every dodge's arc was rewritten at air time 6 to KH1's release hop (5 frames, 6.06), the jump held or not: KH1's variable jump in Sora's button handler (Steam `0x1402A7B30`, from the control step `0x1402A6220`) cuts a rise in state 2 with `+0` flag `0x40000` on and `0x20000` (held) off from 6 frames in to 5 before the apex (`0x1402AF8F0 (actor, 5.0)`), and the dodge kept the flags of the jump before it, let go once (`0x6c003` -> `0x4c003` in the traces). `shared::physics::kh1_held_jump`, after KH1's control step in `takeover::shared_control`: a held hop started that step at physics KH1 (`kh2_hop_held`, the dodge) gets `0x60000` as KH1's own takeoff (`0x1402AFDA0`) sets it, so KH1's own variable jump runs on the dodge: held, the full rise; let go in that window, KH1's release hop and the fall swap at once (KH2's held-dodge rule `0x1403CB680`, same 6 / 5 frames, KH1's hop). Physics KH2 unchanged (`air_step` keeps KH1's cut off, runs KH2's release). Checked (decomp) KH1 Steam and KH2 Steam; Checked (trace). Also (docs only): the dodge's levels Checked (decomp + files) unchanged (LV1 145/18, LV2 155/24, LV3 200/30, MAX 300/36: `0x1403E92D0` record level - 1, `+4` / `+8`); the jump-press handler's glide table is ACTION_GLIDE's (`0x1405CDC38`, Checked (exe) RTTI, t496's assumption settled); a walk-off fall is ACTION_JUMP with the shared enter that gives the dodge (Assumption: no fall action exists in KH2's RTTI), as ours already does; 160 slot frames cover every rise. docs/kh1-aerial-dodge-native.md §7.

### 2026-10-24: Stage 544 (testing branch: Recoil: KH2 hits stopped by a guard on KH1's states recoil natively)
- **A guard that stops a KH2 hit on KH1's states meets it through KH1's own clash.** The tester (plan step 5). `strike_as(.., native)` from `takeover::kh2_hits`: `meet_shot` with the guard's attack object (new `Guard::obj`) against a swing of his KH1 row, so KH1's half `0x1402B4BC0` queues the kind 6 / 7 record on Sora and KH1's hit pass plays his recoil (KH1's or KH2's by the Recoil row); before, `register_wall_hit` went to a controller not driving him. Explosion / Vicinity Break / Magnet Burst are pierce armor in KH2 (Checked (files)), so at KH2 they meet no guard at all. Checked (decomp).

### 2026-10-24: Stage 543 (testing branch: Recoil: the guard repel's 80 frames)
- **KH2's guard repel gives 80 frames without damage (Recoil KH2).** The tester (plan step 4). `combomods::guard_repel` sets KH1's own no-damage timer to 80 (`+0xC` = 80.0, flag 0x80 on / 0x100000 off, as `0x140297F30`), KH2's `+0xD70` = 80 from `0x1403D61D0`. Checked (decomp); Assumption: KH2's step 1.0 a 60 Hz frame.

### 2026-10-24: Stage 542 (testing branch: Recoil: KH2's clash rules on our attacks)
- **KH2's attack kinds on our KH1 rows and KH2's hit shapes (Recoil KH2).** The tester (plan step 3; answers 2 "for now do (c)", 3 "(a)"). `kh1rows::rules` rewrites our rows' data live by the row: at KH2 pierce armor (ATKP kind 1: Guard Break 876, Counterguard 1581, Finishing Leap 854) gets KH1 clash type 6 (meets no attack or guard, hits bodies: KH1's pairing table `0x140528CB8` 1F 09 09 3F 29 18 00 00, Checked (exe); bodies kept from types 1 / 2 / 4 only in `0x1402B3BE0`, Checked (decomp)); refact self 2 (Horizontal Slash, Retaliating Slash) priority at least 0x62 (KH1's half `0x1402B4BC0` doesn't stop it; the enemy recoils by KH1's rule, kind 7 for a gap over 40, answer 3); at KH1 the templates' type and priority (Guard Break no longer passes guards at KH1). Data → object: `0x1402B2DC0` (`+0x38` bits 0..2 → flags 17..19, `+0x3C` → `+0x68`), `0x1402B3270` (Keyblade byte added, capped 99), Checked (decomp). `strike_as` at KH2: pierce meets only `SPECIAL_GUARDS` (empty: no KH1 enemy listed yet) and no attack; refact self 2 isn't repelled by a guard (its owner immune to the swing, KH2's `0x1403D2140`) and meets attacks at priority 99. KH2's ATKP rows Checked (files). Not KH2: a 0x62+ row passes through a guard to the body.

### 2026-10-24: Stage 541 (testing branch: Recoil: Config row, KH2's recoil on KH1's recoil requests)
- **Recoil row; KH2's 41 / 43 on KH1's recoil requests, KH2's push and hop, no protection.** The tester (answers 1 "(b)", 4 "should be part of a recoil switch in the config menu, if you switch to kh1 all moves including kh2 uses kh1's rules exactly and vice versa"). Config > Combat > Recoil (ini `recoil`, default KH1, `styles::Kind::Recoil`). At KH2 `kh2slots::RECOILS` plays KH2's 41 on KH1's 0x6E (slot 72) and 43 on 0x70 (slot 74); 0x6F (KH1's big recoil, slot 73) keeps KH1's animation. `recoil::after_pass` (from `hurt`, after KH1's hit pass `0x14029BBD0`): 0x6E's gap push (`0x1402C7300`) replaced by KH2's 50 back along minus his facing (`0x1403D4650` / `0x1403D4960`, on `+0x120` through `combomods::push_back`); 0x70's hop 20 + gap drift (`0x14029D510`) by KH2's 50 up and 50 × `hit_back_to_jump` back (`0x1403D4800`, `shared::physics::kh2_hop`); KH1's 8-frame no-damage timer (`0x140297F30`: `+0xC`, flag 0x80) put back as before the pass on all three (KH2 gives none). KH1's combo count less one stays (KH2's `+0xD74` the same). Checked (decomp) both games; Checked (files): request table `xa_ex_0010.mset`, 41 / 43 in `P_EX100.mset`. Assumption: one reaction per pass. Drive Forms / borrows keep KH2's controller recoil in both settings.

### 2026-10-24: Stage 540 (testing branch: Recoil: borrowed Keyblade swings meet enemy attacks)
- **The KH1 blade sweep goes on KH2's weapon 1, not the body.** The tester (kh2-recoil-and-parry-system m1, "go ahead"; plan step 1). `combat::Strikes::strike_as` took `h.weapon == 0` (KH2's body) as the right Keyblade, so a borrowed Keyblade swing never met an enemy's melee attack (only shots: the tester's logs) and body windows got the blade sweep. Now weapon 1; body windows and Valor's left blade keep KH2's shapes. Checked (code: `kh2_rig::attack_shapes`, `kh2_data::sora`); Checked (trace: the old logs' "a shot" meetings only). docs/kh1-kh2-recoil.md §6.1.

### 2026-10-24: Stage 539 (testing branch: Config > Combat: Criticals row)
- **Criticals row.** The tester (same thread, answer 2: "make it a config selection (criticals on or off for all moves that call it)"). Config > KH1 in KH2 > Combat > "Criticals: Off / On" (ini `criticals`, default Off, as KH2 has none). Off: in the `kh2damage` hook the attack object's critical chance (`+0x94`, compared with KH1's random roll in `0x1402BFA80`) is 0 for each of Sora's attacks for the call and put back after: no bonus, no record bit 0x20, no journal tally 8; Critical Plus does nothing. On: KH1's critical, KH2 hits included. Checked (decomp). Assumption: "all moves" means Sora's (enemies and partners untouched); the Combat list's eighth row fits as KH1's own 8-row list does. docs/kh1-kh2-attack-data.md §8.

### 2026-10-24: Stage 538 (testing branch: KH2's power in KH1's damage function)
- **KH2's power inside KH1's damage function.** The tester (kh2-attack-data-and-damage-on-kh1s-swings m1: "either a or b you pick which one the player is going to feel a difference in it feeling like kh2", "go ahead"); picked (a), KH1's curve × KH2's power, since KH1's enemy HP is tuned to its curve (§7). `kh2damage.rs` hooks KH1's damage `0x1402BFA80` (Steam, first 15 bytes `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18`): for an attack of Sora's with a known KH2 ATKP (a `land_hit` move, `combat::landing`; our rows, `kh1rows::kh2_atkp`; and now the native swings on KH1's own rows, data bone 0x30 with our slot playing, `kh1rows::native_atkp`) the curve pointer `0x142DE5DC0` points, for that call, at a copy of the curve × KH2's power / 100 × `damage_scale` (× the max-MP Magic stand-in, `combat::magic_factor`), rounded up as KH2; KH1's function then runs whole (critical, min / max, floor / cap, resistance, Aero, mode 2, Second Chance, at least 1). `scale_record` no longer rewrites `+8` or carries fractions while it's in (ini `kh2_damage = off`: the old way; Epic: the old way). KH1's MP charge per hit unchanged (answer 3a). Checked (decomp): the function and its one caller; Checked (exe): the prologue and its single load of the curve pointer at `0x1402BFBB4`. Assumption: (b) would have made late fights much longer (not measured). docs/kh1-kh2-attack-data.md §7.

### 2026-10-24: Stage 537 (testing branch: Combo at KH1: KH2's picks only as ability moves at KH1's count)
- **At Base / Aerial Base Combo KH1, KH2's combo length no longer decides anything.** The tester (same thread, m17: "changing the combat settings back to kh1 doesn't seem to change the row selection ... (double check)"). Checked in the code: plain rows, KH1's request table and the ledge (KH1's own row 2; KH2's drop test off) do switch; but `combomods::attack_row` still took KH2's pick by KH2's length, so with Combo Plus past KH1's length KH2's controller borrowed Sora for a plain KH2 swing, and an ability finisher (Explosion, Aerial Finish, Magnet Burst) could replace a KH1 swing before KH1's finisher. Now at KH1 a pick counts only when it is an ability move of the kind KH1's own count is at (swing before KH1's length, finisher at it); otherwise KH1's row and motion, logged ("not taken, KH1's own row"). KH2-only combo abilities (Finishing Plus, Negative Combo) still make it all KH2's (t488). Checked (code); not tested in game. docs/kh1-move-styles.md.

### 2026-10-24: Stage 536 (testing branch: Guard Repel row: KH1's own block at KH1)
- **Config > Movement > Guard Repel (KH1 | KH2).** The tester (KH2's combo row selection, m17: "(b)", the KH1 setting uses KH1's own reaction when the guard blocks). Ini `guard_repel = kh2 | kh1` (default kh2, today's behaviour). KH2: the repel as t489 (motion 42, pushed back 200, Counterguard). KH1: `combomods::kh1_block` gives Sora what KH1's clash `0x1402B4BC0` gives the owner of KH1's own guard (row 0x3D, data 0x4DA8: bone 0x30, clash type 2, priority byte 0, so the Keyblade's `+0x33` clamped 1..99 by `0x1402B3270`, under 0x62, not 100): a kind 6 reaction `0x14029D720` (request 0x6E, idle queued, 8-frame no-damage timer, combo count −1, pushed back by a negative gap) or kind 7 `0x14029D5F0` (0x6F, no push) when the attacker is more than 40 stronger, run at once with a local record (`+8` the gap), and the guard's result word (combat `+0x50`) marked 0x10 / 0x20, which KH1's Counterattack (`0x140294220`, command 99) reads with 0x6E playing. The attacker's priority from the pool attack our guard object met, or a hit record's data `+0x3C`. Steam-only constants, first bytes checked before each call. Checked (decomp), Checked (exe) for the prologues, Checked (files) for data 0x4DA8; Assumption: an enemy attack object keeps its data's priority; the attacker's own recoil stays KH1's clash on our priority-100 guard objects at both settings. docs/kh1-anim-slots.md §7.x.3.

### 2026-10-24: Stage 535 (testing branch: Physics KH2: KH2's hold with its arc clock running)
- **KH2's no-gravity hold runs the arc's clock (physics KH2).** The tester (m5 answer 1: "(b) unless you have kh1 physics on"). `shared::physics::air_step`: in a KH2 clip's range 3 / 24 on KH1's air state, physics KH2, the arc's time advances and only the move is 0, as KH2's ACTION_JUMP step `0x1403CB360` does (Checked (decomp) KH2 Steam), so he falls at once at the fall cap after the hold: casts, Aerial Dive (46..54), Finishing Leap. Physics KH1 unchanged; KH2's controller (Drive Forms, borrows) unchanged. docs/kh1-magic-native.md §4h.

### 2026-10-24: Stage 534 (testing branch: KH2 magic: KH2's air cast start (40 hop, momentum kept); no clip hover at physics KH1)
- **KH2's air cast start at physics KH2; no clip hover for casts at physics KH1.** The tester (m5 plan item 3; m4: "see how casting is done in the air physics wise to get the same for kh1"). `shared::physics::cast_stops` is now KH2's start (KH2 Steam `0x1403C5170` → slot 4 `0x140409B00` → `0x1403CBC30 (actor, 40)`, Checked (decomp)): a hop of sstm `magic_jump` 40 (Checked (files)) on KH2's arc from where he is, the sideways velocity kept (then `air_step`'s × 0.95 a tick drift in a move); KH1's command step's halving and 30 hop after the spell start are undone by `cast_after_command` (after KH1's command step in `combat::command`, end of `reflect::cast_native`). t487's stop is gone. At physics KH1 `air_step` no longer holds a cast slot in its clip's range 3 (`kh2slots::cast_playing`): KH1's own air cast has no hover (`0x1402AF6A0`, Checked (decomp)). Assumption: KH2's stick target is empty during a cast motion (drift × 0.95). docs/kh1-magic-native.md §4g.

### 2026-10-24: Stage 533 (testing branch: KH2 magic: Reflect on KH1's cast state)
- **Reflect on KH1's own cast state, no borrow.** The tester (same thread, m5 plan item 2, "go ahead"). `reflect::cast_native` (from `combat::command`, base Sora in KH1's own states 0 / 6 / 0x18 / 2, Steam) does what KH1's command step does for a spell (Steam `0x1402A7510` → pre-step `0x1402A38E0`, `0x1402A3600` / `0x1402A3DB0`, Checked (decomp)) with requests of ours (`kh1rows::OURS` "Reflect" / "Reflect (finisher)" / "Reflect (air)": KH2 motions 71 / 72 / 73, new `kh1rows::Kind::Cast` / `CastFinisher` / `AirCast` on KH1's Thunder cast slots 56 / 81, built as cast slots in `kh2slots` so KH2's combo window, cut and magic windows apply): the cast request with KH1's action flag, the idle (ground) or fall 6 blend 8 (air) queued, KH1's air hop `0x1402AF960 (btltbl +0xE8 = 30)` and `+0x3C0 × btltbl +0xF0 (0.5)` via `0x140386CD0` (btltbl pointer Steam `0x142D22D30`), the move direction normalised on the ground (`0x1402A93D0`); KH2's side: cost (`mpcost`), the combo step (`formmagic::note_step` / `take_step`, finisher cast 72 on the ground combo's last), voice, script. `takeover::native_spells` runs `reflect::native_step` on KH1's frames (slot motion and KH1's time; 4 frames' grace for KH1 to start the slot); `reflect::kh1_frame` no longer ends it in KH1's own states. Landing mid-cast is KH1's landing (its slot 11 drawn by the Jump row, the tester's m5 answer 2). Drive Forms, flight (state 8) and Epic keep the borrow. Checked (decomp); Assumption: KH1's Thunder request 0x38 / 0x85 entries are slots 56 / 81 (from t477's file notes). docs/kh1-magic-native.md §4f, docs/kh1-reflect-spell.md.

### 2026-10-24: Stage 532 (testing branch: KH2 magic: Cure KH1 is KH1's whole cast)
- **The Cure row at KH1 casts KH1's whole Cure.** The tester (kh2-magic-and-reflect-native m4: "cure kh1 would cure exactly like kh1 with its effects and cast while the cure switch for kh2 would switch"). With Magic / Aerial Magic KH2 and the Cure row KH1, `combat::command` no longer borrows Sora for Cure (KH1's command step `0x1402A7510` casts it as unmodded), `kh2slots::redirect` leaves KH1's Cure requests 0x39 / 0x86 on KH1's own slots 56 / 81, and `takeover::command_started` no longer puts KH1's Cure on KH2's Cure cast motion's clock (`formmagic::BASE_KH1_CAST` stays off for it): KH1's animation, script, effects, heal and cost. The Cure row at KH2 is unchanged. Checked (code); Checked (files) the request table (from t477's notes). docs/kh1-move-styles.md.

### 2026-10-24: Stage 531 (testing branch: Glide: Donald and Goofy glide after Sora with KH2's Glide)
- **Donald and Goofy glide after Sora with KH2's Glide.** The tester (question in m3: "(b)"). Their AI (`0x1401CF9E0`, members 1 / 2, from `0x1402A3300` → `0x1401CDBD0`) glides after his jump with KH1's Glide / Superglide only when Sora's ability bits (Steam `0x142D5EC10`, rebuilt by `0x1402A6380` on a menu close / map setup) have `0x80` / `0x100`. `glide::party`: at Glide KH2, base Sora with KH2's Glide and neither of KH1's, `0x80` set each Sora physics frame; taken off (only if ours, and he lacks KH1's Glide) when he can't glide with KH2's. Their glide stays KH1's. Sora's other reader of `0x80`, the fall state's jump press, is ours since t526 (no KH1 glide on his jump). `takeover::kh1_has_glide`. Checked (decomp) every read of the two bits. docs/kh1-glide-native.md §8.6.

### 2026-10-24: Stage 530 (testing branch: Glide: the borrow taken out, Square starts KH2's glide in KH1's state)
- **Square starts KH2's glide in KH1's glide state; the borrow is gone.** Same ask (plan step 5). `shared::control`: Square in the air at Glide KH2 calls `glide::start`; the jump in KH2's glide with the Aerial Dodge left ends it into the fall and starts the native Aerial Dodge (KH2's shared air slot `0x1403CB9F0`); in 0x15 at Glide KH2 KH1 sees neither the jump nor Square. Out: `shared::GLIDE`, `takeover::GLIDE_BORROW`, its launch hand-back and the dodge out of the borrow (t496, `DODGE_OUT_OF_GLIDE`), `combomods::native_dodge_built`. KH2's controller no longer glides base Sora in KH1's states (its Glide level kept aside, `KH2_GLIDE_KEPT`); Drive Forms keep theirs. Steam only. Checked (code / decomp). docs/kh1-glide-native.md §8.5.

### 2026-10-24: Stage 529 (testing branch: Glide: KH2's end and release in KH1's glide state)
- **KH2's end and release in KH1's glide state.** Same ask (plan step 4; walls question 2 "(b)", and KH2's glide has no wall rule). State 0x15's button check (Steam `0x1402B2BF0`) wrapped for Sora: Square let go ends the glide or hover at once (no 46-frame minimum); its transition (`0x1402B2780`): Neverland's flight first as KH1's (world flag `0x20`, `0x1402B1DA0`, state 8), then on the ground or under KH2's end height (growth `+0x38`, `0x1404235B0`) the end, KH1's wall end (moved under 70%) not run. `glide::end` (KH2's `0x1403CBC30`): KH1's fall from rest `0x1402AF8F0 (actor, 0)`, his velocity kept in `+0x3C0` (no x0.7, no hop of 8), KH1's fall request 6 with KH2's blend 16, KH2's launch flag cleared for the air time; on the ground KH1's fall state lands him the next frame. Checked (decomp); Assumption: KH2 ends on the release at once (not found in the glide action's own functions). docs/kh1-glide-native.md §8.4.

### 2026-10-24: Stage 528 (testing branch: Glide: KH2's turn, hover and motions in KH1's glide pre-step)
- **KH2's turn, hover and motions in KH1's glide pre-step.** Same ask (plan step 3; hover as a sub-mode of 0x15, the tester's question 1). State 0x15's pre-step (Steam `0x1402B2880`) wrapped for Sora: the hover picked with the stick neutral at the start (`0x1403A8369`), the stick takes it into the glide with the launch speed back (`0x140423840`), never back; the turn toward the stick at growth `+0x3C` a frame (`0x140423B90` / `0x140423BC0`) through KH1's facing set `0x140297E10`; KH2's motion pick (`0x140423A70`): 205, 206 / 207 past dot 0.98 by the side of the turn, 208 in the hover, blend 16, through KH1's request `0x14029F7C0`; KH1's water check `0x1402C12B0` still last. `kh2slots::GLIDES`: 205 / 206 / 207 on KH1's glide requests 0x71 (0x72) / 0x73 / 0x74 at Glide KH2 (slots 75..78 as templates); `kh1rows::OURS` "Glide hover" (208, request only, KH1's air swing template), its time put back to 0 at its end. Checked (decomp / files); Assumption: `0x140142120` is facing x stick (which side is 206), and KH2 loops 208. docs/kh1-glide-native.md §8.3.

### 2026-10-24: Stage 527 (testing branch: Glide: KH2's speed and fall in KH1's glide step)
- **KH2's glide speed and fall in KH1's glide step.** Same ask (plan step 2). State 0x15's step (Steam `0x1402B2A20`) wrapped for Sora: on KH2's physics tick (every two 60 Hz frames, dt 2, `0x1403B89A0`) the glide's velocity `facing x ((1 - a) * speed + a * |v|)` (`0x140423920`, growth `+0x30` / `+0x20`), the hover's x0.9 (`0x140423650`), the fall `(height - +0x28) * +0x24` capped at `+0x2C` from KH1's own `+0x3D4` (`0x1404237A0`); the displacement handed to KH1's mover (walls: he slides, KH2's glide has none), `+0x3C0` / `+0x3C4` KH2's velocity. KH1's +0.4 ramp and saw-tooth fall don't run for him. Checked (decomp); Assumption: KH2's collision doesn't cut `+0x20` on a head-on wall. docs/kh1-glide-native.md §8.2.

### 2026-10-24: Stage 526 (testing branch: Glide: KH2's glide start in KH1's glide state)
- **KH2's glide start, enter and exit in KH1's glide state 0x15 (not started yet).** The tester (kh2-glide-in-kh1s-glide-state: "(b)", "go ahead", plan step 1). New `glide.rs`: `glide::start` takes base Sora from KH1's jump / fall state into 0x15 with KH1's set-state (Steam `0x1402B1960`) above KH2's start height (growth `+0x34`), no KH1 timer read, his speed not halved, KH2's stored launch speed put back when KH2 has one (`takeover::launch_kept`, KH2's enter `0x1403CBD30`), else his speed as it is; KH1's request 0x71 with KH2's blend 16 (Steam `0x14029F7C0`). Wrappers on KH1's state table (Steam `0x142DB3840`, layout read from `0x1402B1B80`: `+8` enter, `+0x10` exit, `+0x18` pre-step, `+0x20` step, `+0x28` transition, `+0x30` button check, `+0x38` jump press), for Sora at Glide KH2 only (Donald and Goofy run KH1's): enter `0x1402B2D00` / exit `0x1402B29F0` keep `+0x1CC` bit 0 as KH1's but set the combat state's `+0x100` to 0 (no 46-frame minimum, no 24-frame wait: none in KH2's `ACTION_GLIDE` / `ACTION_GLIDEHOVER`); the fall state's jump press `0x1402AFD60` no longer starts KH1's Glide on him (Neverland's flight kept). `takeover::kh2_glide_growth`, `launch_kept`, `launch_used`. Not wired to a button until t530. Checked (decomp) both games. docs/kh1-glide-native.md §8.1.

### 2026-10-24: Stage 525 (testing branch: Guard: KH2's angle test for its guard windows)
- **The guard's coverage is KH2's ATKP angle test.** The tester (same thread, question 1: "go with whats most native and 1:1"). KH2 (`0x1403D1F40`, Checked (decomp)): a guard holds an attack when its ATKP reflect range is 0 or the attack's direction against his facing (turned by the reflect angle x 2) is below -cos(range); KH2 has no Guard Plus (the OG rewrite's `guard_all_round` is ours, never set). Sora's ATKPs (Checked (files)): 161 (guard press) 120, 162 (repel) 180, 406 / 124 / 455 0; angles 0. Before: 90 degrees each side for Keyblade guards. `hurt::guard_faces` (KH1's states and borrows; direction from the record's push `+0x10`) and the KH1 guard objects' data (`combat::guard_data`: `+0x64` cos(range), `+0x60` angle; KH1's directional test `0x1402B4640`, Checked (decomp)). Assumption: KH1's `+0x60` turn sign (unused). docs/kh1-roll-guard-native.md §7.

### 2026-10-24: Stage 524 (testing branch: Counterguard: KH1's Counterattack command during KH2's repel)
- **KH1's Counterattack command in Attack's place during KH2's guard repel.** The tester (same thread, question 2: "(a)"). KH1's swap `0x140294220` writes Attack's entry (`0x140528914`) each refresh, then calls the battle-menu fill `0x1402944C0` (Checked (decomp)); after that fill (`drive::fill`) `combomods::counter_entry` turns an Attack entry of 1 into 99 while our slot of KH2's 42 plays on the ground with Counterguard equipped (`counter_open`; no target needed, as KH2). Choosing it, `combat::command` makes the record KH1's Attack (type 1, target kept), so KH1's Attack step runs KH2's Counterguard through the repel's combo window (t489), not KH1's row 0x3E. Context commands (Talk) and the world entry 0x62 stay. Checked (decomp); Assumption: the fill runs every frame the menu is up. docs/kh1-roll-guard-native.md §6.

### 2026-10-24: Stage 523 (testing branch: Dodge Roll: KH2's roll on KH1's own roll)
- **KH2's dodge roll runs on KH1's own roll, no borrow.** The tester (dodge-roll-guard-and-counterguard-native m1 answer 3: "make dodge roll its own slot ... use kh1s as a base so its native"; "go ahead" after m2). With Dodge Roll KH2, Square on the ground with the stick pushed and KH2's Dodge Roll equipped (after the guard press, stick at rest, didn't take it: KH2's order `0x1403A81F0`) runs KH1's roll start (Steam `0x1402C9E60`) facing the stick, speed 0, KH1's 12-frame no-damage timer it sets (`0x140297F30 (actor, 12.0)`, flags `0x80 | 0x100000`) put back (`combomods::roll_press`). `kh2slots`' roll spec points request `0xDC` at our slot of KH2's 209 on template slot 117 at KH2: `0xDC` has KH1's travel flag (`0xA075`, Checked (files)), so the baked slot carries KH2's root travel; KH1's floor sounds and foot effects rescaled; KH1's "may be left" from KH2's fall-check event (34) in place of KH1's 30. `hurt` evades hits under the level's `dodge_roll_star_time` (10 / 16 / 24 / 32, `combomods::roll_invulnerable`, `takeover::kh2_dodge_roll_growth`). Square held through it: KH2's Quick Run at KH2's check frame. Borrow kept as fallback (forms, slot not built). Checked (decomp) `0x1402C9E60` / `0x140297F30`, Checked (files) request `0xDC` / slot 117; Assumption: KH1's ground step uses `+0x2C` under the action flag, nothing else reads `0x100000` as rolling; KH1's own untouchable flag `0x40` source still not found. docs/kh1-roll-guard-native.md §5.

### 2026-10-24: Stage 522 (testing branch: L2 Stick Menu row)
- **L2 Stick Menu row: L2 + the right stick is the d-pad, with either camera.** The tester (kh2-camera-fixes m14: "1. (a) but should be a toggle in the camera menu that kh1 camera and kh2 camera can use"). KH2's pad (Steam KH2 `0x14039c180`) turns the right stick past its dead zone (60 from 127.5) into raw bits 16..19; its static remap table (`0x1405c3420`) maps only the left stick's 20..23 onto the d-pad, so the right stick's d-pad use under L2 comes from the run-time PC table. Config > Camera: "L2 Stick Menu: Off / On" under L2 Camera Hold (ini `l2_stick_menu`, default off). With it on and L2 held, `camera::dpad_from_stick` (from `probe::pad`, on each pad record `0x140112C70` writes) adds the d-pad bits for the right stick past KH2's dead zone (diagonals give two) and centres that record's right stick, so KH1's pad step, command menu, repeat and Drive page flip take it as the d-pad and KH1's camera doesn't turn; KH2's camera drops its right stick and mouse under the same test (R3 stays L2 Camera Hold's). KH2's first person, the mouse-as-d-pad and the keyboard's camera keys aren't changed. Checked (decomp) KH2's direction bits and dead zone, Checked (exe) the static remap table; Assumption: KH2's run-time table pairs the right stick's bits with L2 onto the d-pad (the tester's description); KH1's repeat timing is used, not KH2's. docs/kh2-pad.md.

### 2026-10-24: Stage 521 (testing branch: Hit effects: KH2 spells call their own hit spark)
- **KH2's spells show their own hit spark, through KH1's own lookup.** The tester (answer 2: "(a) if a vanilla magic attack calls for its vanilla spark then let it, if its from kh2 let it call its own spark"; answer 1: Keyblade swings unchanged for now). `combat::SpellAttack::package` (formmagic's KH2 casts and Wisdom's shots, Reflect's pop) goes to `land_hit` with the ATKP row's effect_on_hit / random_effect: the attack object is made from a copy of its KH1 data (`hit_data`, one 4 KB page near the battle data via `kh1rows::near`, 32 slots) with `+0x28` = effect_on_hit - 1 (+ KH2's random step; -1 for 0: no spark, as KH2 `0x1403d1b40`), and its `+8` is the spell package's DPX (`kh2dpx::hit_list`, which writes the tagged entries' masks in KH1's numbering: KH2's kind 4 on KH1's 2, KH2's wall kind 2 dropped; entries whose effect runs programs KH1 lacks are unfindable, and if that is the one the hit would find KH1's spark plays instead). KH1's contact `0x1402b4060` -> `0x1402b4320` -> `0x1401dffe0` -> `0x1402ae650` then picks, places and turns KH2's element. KH1's own spells keep KH1's sparks (t345); with `kh2_effects` off or the package not built, KH1's spark as before. Checked (decomp) KH1's contact, lookup and record fill (kind 0x80 damage effect from the attacker's list, none in KH2 lists, as KH2 plays none there); Checked (files) the spell packages' hit elements. Assumption: a KH2 spell attack carries its spell file's PAX (Wisdom's shots the form archive), not traced to the line that sets KH2's attack `+0x28`. `docs/kh1-kh2-hit-effects.md` §6.3, §8.

### 2026-10-24: Stage 520 (testing branch: Hit effects: KH2's PAX lookup word kept, as KH1's DPX masks)
- **KH2's element lookup word kept and written as KH1's effect-list masks.** The tester (kh2-hit-effects-on-enemies, go-ahead on the plan). `kh2_formats::pax::PaxElement::tag` keeps the `+8` word KH2's lookups search (`0x1402c6730`, Steam KH2) instead of dropping it; `kh1dpx::Variant::tag` writes its low half into the built DPX entry's `+0x10` (KH1's kind mask) and its high half into `+0x14` (group mask), the masks KH1's own lookup `0x1401dffe0` (Steam) searches; `kh2dpx` passes each element's word through and logs it. Nothing plays differently yet (no KH1 lookup reads our DPXs). Checked (decomp) both lookups; Checked (files): every KH2 DPX entry checked has 0 in those words, and the spells' hit elements carry kind 1 with groups 1 / 2 (`docs/kh1-kh2-hit-effects.md` §6.1, §6.2). Test `element_tags_become_kh1_masks`.

### 2026-10-24: Stage 519 (testing branch: Sleeves thread: the coat's hem hangs from the hips)
- **The coat's top stays on the chest, its hem lies on the hips.** The tester (missing-sleeves-animation, after t500: "the coat is still sticking out on kh2 animations" / "don't worry about the kh2 model for now"). `examples/coat_check.rs` now measures how far the coat's lower joints stand off the hips' up axis against the bind (and the same over KH1's own motions, `kh1 <mset>`): carried whole by the chest (t491) they stood more than 2 out in 42% of KH2's frames (KH1's own coat: 15%), the back 6.0-6.6 out behind the hips and the +x front 7.3-8.2 into them through the whole idle, walk and run (KH2's chest leans forward of its hips). `retarget::coat_low` (live and baked): each chain's root and root + 1 stay rigid on the chest as KH2's jacket; root + 2 goes halfway, the tip all the way to where KH1's hips (3) carry it in the bind, segments re-aimed, lengths kept (`COAT_LOW` 0.5 / 1.0). After: out >2 0% (max 1.5), into the hips >2 10.1% (KH1's own 12.7%), nearer a thigh than the bind 14% (was 22%); front tips inside the shorts 0.57% of joint-frames (was 0.17%; kicks, the run at most 2.3), no pop above the knees' in any motion. Checked (files): 83 motions of `P_EX100.mset`, every half frame. Assumptions: the shorts' radius from KH1's panel bones; KH1's own motions in plain FK (IK helpers not evaluated). Seven other rules measured (table). Test `local_own` (coat top rigid, hem never over 2.5 off the hips, lengths kept). docs/kh1-secondary-motion.md "The coat's lower half".

### 2026-10-24: Stage 518 (testing branch: Reflect: the barrier's blocking shapes at KH1 Sora's size)
- **Reflect's blocking area at KH1 Sora's size.** The tester (same thread, m18 answer 2: "if sora is smaller than kh2 sora then yes scale it down"). KH1 Sora is 0.89 of KH2 Sora (hips 78.9 against 88.5 in the two bind poses, `Retarget::hip_ratio`, Checked (files)); `reflect::kh1_sized_barrier` scales the barrier's attack shapes (ATKP 573) about Sora's place by `pose::kh1_size` (centre offsets, radius, height) before `Strikes::barrier_meet` meets enemy attack objects with them, as t483 scaled its effects. `hurt`'s stop of hits while the barrier stands tests no shape (unchanged); the counter / pop shapes stay KH2's. docs/kh2-effects.md §18.22.

### 2026-10-24: Stage 517 (testing branch: KH2 effects: KH2's object frame is the actor's rows (half turn on KH1's bound frame))
- **Reflect's ground barrier on his raised hands.** The tester (same thread, m18: "reflect on the ground is the one currently not placed correctly"). KH1's actor matrix (`0x1402A0A40`): `+0xD0` = facing × (0, 1, 0), `+0xF0` = 0 − facing (`+0x30`), `+0xE0` = `+0xF0` × `+0xD0`; the skeleton pass `0x140391C70` (from `0x14029F0A0`, given `actor + 0xD0`) draws the model with rows 1 and 2 negated, so the rows are the model's (x, down, back), KH2's object frame exactly; §18.21 had `+0xF0` as ahead (Checked (decomp)). KH1's 0x8000 frame read straight times its apply's `P` is the model's (−x, down, forward): KH2's turned half round about y. The t515 log's origin line for Reflect 3's element 0, (19.9, −76.8, 16.4) along the rows against KH2's (−19.6, −75.7, −16.2), agrees (Checked (trace)); the files put those pieces by his hands in motion 71 (Checked (files)). `kh2dpx`: bound copies on 0x8000 / 0x8005 take `H · t`, `H · R` (`HALF_TURN_Y`); position-only offsets go through `P · H` (`KH2_IN_KH1_OBJECT`). Also changes every object-root piece with an x / z offset or a turn (Explosion, Guard Break, Finishing Leap, Aerial Finish, forms' root effects, 0x4001 / 0x4009). Assumption (open): 0x8005 reads 0x8000's frame. docs/kh2-effects.md §18.23.

### 2026-10-24: Stage 516 (testing branch: KH2 magic: the cast motion for the spell script's first frame)
- **KH2's spells come out on the first press again.** The tester (kh2-effects-in-kh1s-own-effect-system, m18: "kh2s magic doesn't get casted the first time you input said magic"). Since t477 KH2's cast runs in KH1's own cast state: `formmagic::kh1_spell_start` (KH1's spell start `0x14028F140`) starts KH2's script, but KH1 only queues the cast's request there and plays it from its next animation step, so the script's first frame saw Sora still in his idle (or our slot at a time below 0). KH2's spell scripts end a cast at once when the caster isn't in the cast motion on their first run: `FIRE_3` / `BLIZZARD_3` / `THUNDER_3` run with `kh2-magic` and a first-frame motion other than 56 end that frame with no release and no attack, with 56 they release and attack (Checked (files)); the t515 log and traces show every combo's step 1 with no hit and steps 2 on hitting 24..26 frames in, the motion already playing (Checked (trace)). `formmagic::shots_step` now gives the scripts' world (`KhWorld`) Sora in the cast motion until the cast is seen playing (`casting`'s `SEEN`); KH1 is untouched. docs/kh1-magic-native.md §4.

### 2026-10-24: Stage 515 (testing branch: KH2 sounds: kh2_sound_engine = mod keeps KH1's engine out)
- **`kh2_sound_engine = mod` keeps every KH2 bank out of KH1's engine.** Found with the startup crash (same thread): `kh1bank::frame_end` registered the banks and the voice bank whatever the ini said; only the plays followed it, so the switch-back wasn't one. It now returns at once unless the engine setting is kh1. Checked (code).

### 2026-10-24: Stage 514 (testing branch: KH2 sounds: the voice bank's registration gets its name (startup crash))
- **Startup crash: KH1's voice-bank registration was missing its name.** The tester (kh2-sounds-through-kh1s-sound-system: "crash when starting the game ... if you find the reason I give you permission to fix it. go ahead"). t513 called KH1's `0x1400E0110` with three arguments; it takes four, r9 the entry's name, used as the format of `sprintf(entry + 0x30, name)` (`0x140051260`) before the entry is linked (Checked (exe): `0x1400E0132`, `0x1400E0279..0x1400E0280`; the decomp shows three). KH1 formatted whatever r9 held as soon as its sound system was up. KH1's own voice path `0x1400DFE60` takes the name in r9 too (Checked (exe)). Now `kh2_tt0_sora.win32.scd` is passed (≤ 0x20 bytes, the 0x50-byte entry), empty / oversized banks skipped, a busy lock retried next frame. Assumption: this was the tester's crash (no log sent).

### 2026-10-24: Stage 513 (testing branch: KH2 sounds: battle quotes through KH1's voice path)
- **KH2's battle quotes through KH1's voice path.** The tester (question 2: "what ever is closet to how kh1s systems handle it"). KH2's voice bank goes into KH1's voice list (`0x1400E0110`, a free bank number from 127 down); a quote is picked by KH2's rule (`0x1403CDB60`) and asked of KH1's `0x140290930 (Sora, clip, VTBL priority)` with `0x142D37270[member]` swapped for the call: KH1's priority, channel `0x142D371D0`, mouth motion and voice bus. Old player the fallback. Checked (decomp); Assumption: KH1's voice create plays KH2's Vorbis clips.

### 2026-10-24: Stage 512 (testing branch: KH2 sounds: buses 4..8 on KH1's effects bus)
- **KH2's buses 4..8 on KH1's effects bus 2.** The tester (plan step 3). KH2's effects slider (`0x140138290`) drives buses 2 and 4..8 alike, KH1's (`0x1400DDD10`) 1 / 2 / 3 (5 / 6 to a scale), never 4, 7 or 8: cut banks on 4..8 (the Keyblade bank se500's 8) go on bus 2. Checked (decomp).

### 2026-10-24: Stage 511 (testing branch: KH2 sounds: effects through KH1's effect queue at Sora)
- **KH2's effects through KH1's effect queue at Sora.** The tester (plan step 2; question 3 "same as 2"). Queued with KH1's `0x140179AC0 (no, id, Sora+0x10, 0)`; KH1's queue step `0x1401791E0` creates, pans and plays them; our slot is stopped with KH1's `0x140179D90` once its length has played in game time (`0x1401789F0`), as the step never frees slots (Checked (exe)). The old player is the fallback (not Steam, bank not loaded, queue full). Ini `kh2_sound_engine = kh1 | mod` (default kh1). Checked (decomp); Assumption: the engine plays the cut banks.

### 2026-10-24: Stage 510 (testing branch: KH2 sounds: banks registered in KH1's engine)
- **KH2's sounds registered as KH1 banks (nothing plays through them yet).** The tester ("go ahead", plan step 1). `kh1bank.rs`: each sound a `se9NNNNN.win32.scd` bank registered with KH1's `0x1400E0F00` on a copy from KH1's `_aligned_malloc` (IAT `0x1403B0778`; KH1 frees banks with `_aligned_free`), put back after KH1's bank list `0x1404D65D8` changes (`0x1400D2C70(1)` → `0x1400DBAC0` releases all, from the map tear-down steps), only while holding KH1's adding semaphore (`[0x1421AAFD8]+0x18`) and with more than 128 slots left (`0x14004F570`); logged ("KH1 banks:"). Checked (decomp); "room unload" an Assumption.

### 2026-10-24: Stage 509 (testing branch: KH2 sounds: one-sound banks in KH1's file form)
- **One KH2 sound cut into a one-sound bank, KH1's own file form.** The tester (kh2-sounds-through-kh1s-sound-system m1: "1. a"). `kh2_formats::scd_cut`: a one-sound SEDBSSCF bank (record, tracks, layout, attributes, audio; renumbered, laid out as KH1's own); KH1 reads SCD versions 1..4 (`0x140078D40` / `0x140078DA0`, Checked (decomp)). Cutting sound 0 of KH1's own `se021010` / `se017113` / `se021073` gives them back byte for byte; every playable KH2 id decodes the same except 35 se000 ids our old reader played from the wrong audio (index − 152: Magnet Burst 169, Thunder 126..128 / 148, Reflect 138 / 139 / 176), Checked (files). docs/kh1-sound-native.md.

### 2026-10-24: Stage 508 (testing branch: Hit reactions: Medium Hits row)
- **Medium Hits row.** The tester (answer 2: "how sora receives knock back should also be a config"; answer 3). "Medium Hits: Blow / Flinch" (ini `medium_hits`, default Blow): at Flinch, KH1's kind 4 (20..29% of HP, `0x1402BF290`) becomes kind 3 before the pass on KH1's states with Hit Reactions KH2; in base Sora's borrows the controller flinches. Re-hit: KH2's rule (`0x1403CF600` / `0x1403CF230` / `0x1403D4880`) and KH1's (`0x140297F30` / `0x140297FE0`) both give 30 frames after the reaction ends; only KH2's same-group exception differs, which KH1's attacks can't trigger, so KH1's timer is kept. Checked (decomp).

### 2026-10-24: Stage 507 (testing branch: Hit reactions: KH2's push on KH1's push)
- **KH2's push on KH1's push (Knock-back Numbers row).** The tester (answer 1: "make it a switch in the config ... your judgement"). `hitreact.rs`: after the pass, KH1's flinch push speed (`0x1402C7300`) is KH2's 0.1 × distance, and each frame KH1's push step `0x1402C7070` moves him by KH2's speed with KH2's ×0.9 decay (`0x1403D4960` / `0x1403D4570`); the air flinch gets KH2's hop 50 / drift × 0.048 (`0x1403D4800`) through `kh2_hop`. Config "Knock-back Numbers: KH1 / Scaled / KH2" (ini `knockback_numbers`, default KH2, as `0x1403CEAB0` falls back to sstm's 200 for attacks with no numbers). btltbl block Steam `0x142D22D30`. New three-value row type `config::Item::Pick` (Assumption: its panel layout). Checked (decomp).

### 2026-10-24: Stage 506 (testing branch: Hit reactions: KH2's flinches on KH1's reaction requests)
- **KH2's flinches on KH1's reaction requests (Hit Reactions row).** The tester (soras-hit-reactions-native m1: "go ahead"). Config > Combat > Hit Reactions (ini `hit_reactions`, default KH1). At KH2, `kh2slots::REACTIONS` plays KH2's 10 / 11 / 12 / 13 on KH1's ground flinch requests 0x48..0x4B (slots 61..64) and 14 / 15 on the air flinches 0x4E / 0x4F (67 / 68); KH1's reaction functions and the hit pass `0x14029BBD0` still run the timer, push, voice, HUD and MP; KH2's free state from 50 is KH1's "may be left"; KH2's clip voices left out. Checked (decomp): KH1's 0x48 is front by KH2's own test (`0x1403D2FE0`); Checked (files): both msets' lengths. Knock-backs wait for KH2's blow state. docs/kh1-hit-reactions-native.md.

### 2026-10-24: Stage 505 (testing branch: Quick Run: KH2's start effect above LV1)
- **Quick Run's start effect above LV1.** Same ask (the research's open point). KH2's start calls `0x1403B6670(actor, level + 0x6C)`, a PAX element by id (`0x1402C6B30`): elements 110 / 111 / 112 of `P_EX100.a.us` for LV2 / LV3 / MAX, started through `kh2fx::event`; 203 joins `kh2fx::DEFAULT_MOTIONS`. Checked (decomp / files).

### 2026-10-24: Stage 504 (testing branch: Quick Run: invulnerable frames 1:1)
- **Quick Run's invulnerable frames 1:1.** The tester (question 1: "make behave exactly 1:1 with kh2 so if its (a) then do it"). Stage 104's 6-frame minimum (`QUICK_RUN_STAR_MIN`) is gone: KH2's fmab 0 / 8 / 12 / 16 by level, and `hurt` stops hits during the native run's frames (`quickrun::invulnerable`). Checked (files); Assumption: KH2's counter `+0xD70` counts 60 Hz frames.

### 2026-10-24: Stage 503 (testing branch: Quick Run: off a ledge)
- **Quick Run off a ledge.** Same ask. KH2's slot 4 (`0x140409B00` → `0x1403CBC30(actor, 0)`): a fall from rest keeping the velocity, the Aerial Dodge left (AIRSLIDE isn't airborne, its `+0xC` bit 0 clear in the exe's action table `0x140750D30`). Ours: KH1's fall from rest as `0x1402AF8F0` writes it, state 2 set by our transition (so the physics step's 8-unit walk-off cap doesn't run), the slide's velocity in `+0x3C0`, fall request 6 blend 8. Checked (decomp / exe); Assumption: the fall motion (KH2's fall action asks none).

### 2026-10-24: Stage 502 (testing branch: Quick Run: the start points start the state)
- **Quick Run's start points start KH1's Quick Run state, no borrow.** Same ask. `shared::control`: Square held through KH1's roll (its roll flags `0x80 | 0x100000` cleared as KH1's think does, `0x140297FE0`), Square while moving without KH1's Dodge Roll, and KH2's action button picking Quick Run at Dodge Roll KH2 (KH2's order `0x1403A75A0`: guard at rest, roll, Quick Run) call `quickrun::start`. The borrow stays the fallback (forms, Epic, requests not built). Checked (decomp).

### 2026-10-24: Stage 501 (testing branch: Quick Run as KH1 movement state 0x2A)
- **KH2's Quick Run as KH1 movement state 0x2A.** The tester (quick-run-native: "do your first purposed route ... make behave exactly 1:1 with kh2"). `quickrun.rs`: state 0x2A in KH1's state table (Steam `0x142DB3840`, flags 0x08, our step and transition): KH2's start `0x1403A9830` (fmab speed, brake, time, stop brake, invulnerable time; `takeover::kh2_quick_run_growth`), hold timer and stop `0x140423500` (counts while Square held, 0 on release; stop brake and motion 204), the velocity step on KH2's 2-frame physics tick (`0x1403B89A0`), the move through KH1's ground step (Steam `0x1402B1850`), `+0x3C0` the velocity; motions 203 / 204 as request-only `kh1rows` rows; KH1's landing at 204's end. Checked (decomp) KH2 and KH1, Checked (files) 203 / 204; Assumption: 204's end into KH2's landing. docs/kh1-quick-run-native.md §6.

### 2026-10-24: Stage 500 (testing branch: Sleeves thread: coat research tool)
- **Coat research tool.** Same ask. `kh1-data/examples/coat_check.rs` measures the chest-rigid coat (t491) against eleven candidate rules over 83 motions: none removes its brief clips at the top of the thighs (0.15% of half frames, side kicks / splits / spins) without one-frame pops or more clipping, so the coat stays as t491. No behaviour change. Assumption: the shorts' radius from KH1's panel bones. docs/kh1-secondary-motion.md 'The coat against the thighs'.

### 2026-10-24: Stage 499 (testing branch: Sleeves thread: thighs and shins roll as KH2's skinned leg bones)
- **Thighs and shins roll as KH2's skinned leg bones.** The tester (missing-sleeves-animation, after t491: "keep researching until we have a good looking animations that emulate kh2s animations as close as possible while still making it look natural to kh1's model"). KH2 draws its thighs on the shorts' leg bones 24 / 53 and its shins on the knees 12 / 41 (Checked (files): `P_EX100.mdlx`); the retarget rolled them by the knee's direction, up to 109° / 62° (thighs) and 44° / 69° (shins) off those bones (motions 8, 7, 169, 153, 40, 183, 204, 42, 171). `retarget::Pole::Knee`'s `skin`: each limb aimed along KH2's joints and rolled as its skinned bone (turn from bind on the part's bind z), the knee's direction only where that z nears the limb's line. After: at most 0.2° (Checked (files), 83 motions every half frame, `examples/leg_roll.rs`); largest half-frame turn = KH2's own. Live and baked; `inverse` (KH2 model, off) keeps the knee rule. docs/kh1-secondary-motion.md.

### 2026-10-24: Stage 498 (testing branch: L2 Camera Hold row)
- **L2 Camera Hold row.** The tester (kh2-camera-fixes m13: "2. (a) but make it a toggle in the cameras config menu"). Config > Camera: "L2 Camera Hold: Off / On" (ini `l2_camera_hold`, default off): with it On and L2 held, KH2's follow and lock-on cameras take no right-stick turn or zoom, no mouse motion and no R3 reset, as KH2's L2. L2 is read as `camera::r2_held` reads R2: KH1's raw pad word `0x14298C9B4` bit `0x100` (Steam). First person, KH1's soft-target drop (`0x1402B11B0`) and Frame Advance (R2 only) untouched; no stick-as-d-pad yet (asked). Checked (decomp / exe) KH2's camera bit tests `0x140178AB0`..`0x140178B80`, masks `0x14071AA98` / `0x14071AA88`; Assumption: KH2 ties L2 to them in its run-time remap table `0x140BF3330`. docs/kh2-pad.md.

### 2026-10-24: Stage 497 (testing branch: Lock-on side kick, KH2's rule)
- **Lock-on side kick, KH2's rule.** The tester (kh2-camera-fixes m13: "1. (a)"). KH2's lock-on camera (Steam KH2 `0x140176EC0`, screen test `0x140176710`) tests the target first, Sora (100 above his feet) only if the target is on screen: off by the left edge (bit 4) the goal yaw is base + 20°, else off by the right (bit 8) base − 20°, top / bottom only none; left wins. Ours took the sign from the way the camera was turning. `kh2_camera::lock_side` is KH2's rule; the crate's frame is KH1's with y flipped, so its screen sides are mirrored and KH2's bit 4 is the crate's `screen::RIGHT` (`kh1-render.md` §1.3). Checked (decomp) the rule and base yaw `0x140176A70`; Assumption: KH2's screen handedness is KH1's (a left / right trace settles it). docs/camera-model.md.

### 2026-10-24: Stage 496 (testing branch: Aerial Dodge out of KH2's glide, native)
- **The Aerial Dodge out of KH2's glide goes native.** The tester (aerial-dodge-native m1, question 2: "(b) full native, glide will get converted"). In KH2's glide's borrow, a jump press with the dodge left ends the borrow before KH2's step (`shared_give_back` into KH1's fall state, the glide's launch kept) and starts `combomods::aerial_dodge_with` on KH1's air state (numbers from KH2's controller while held). Only with our dodge request built (`native_dodge_built`), else the controller's dodge as before. KH2's `0x1403CB9F0` in the air actions' tables Checked (decomp); which is the glide's: Assumption. Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 495 (testing branch: Aerial Dodge: the early release)
- **The Aerial Dodge's early release.** The tester (aerial-dodge-native m1, plan item 4). KH2's dodge is holdable: bit `0x10000` → the shared enter's held jump `0x20000` (`0x1403CBD30`), release marked by `0x1403D6FC0`, cut by `0x1403CB680` (fall swap `0x1403CB610`, then the release hop), Checked (decomp). The native dodge's hop is held (`shared::physics::kh2_hop_held`), `air_step`'s existing early release cuts it at physics KH2 and asks KH1's fall at once (`combomods::dodge_fall`, shared with the apex). Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 494 (testing branch: Aerial Dodge: its guard as KH1 guard objects)
- **The Aerial Dodge's guard (ATKP 406) as KH1 guard objects on the native slot.** The tester (aerial-dodge-native m1, plan item 3). `takeover::kh2_hits` takes 202's body guard window too (others only on the Keyblade), so `combat::guard_step` makes the all-round KH1 guard object (priority 100) KH2's controller made for its dodge: enemy attack objects meeting it are stopped, no reaction (`refact_self` 2). Hits his hit pass finds are still `hurt`'s. Checked (files): 202's window and ATKP 406. Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 493 (testing branch: Aerial Dodge: the fall at KH2's apex)
- **The Aerial Dodge's fall at KH2's apex.** The tester (aerial-dodge-native m1: question 1, my pick; plan item 2). `combomods::dodge_apex` (before Sora's KH1 physics frame): with our dodge slot playing in KH1's air state, 5 frames before the apex by KH1's arc fields (`+0x3B0` + 5 >= `+0x3B8`) and nothing pending (`+0x178`), KH1's fall request 6 with blend 8, action flag off. KH2's rule `0x1403CB680` / `0x1403CB610` (motion 4, blend 8.0) Checked (decomp); KH1's apex swap `0x1402AF630` only for its own requests, Checked (decomp). Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 492 (testing branch: Aerial Dodge on KH1's air state)
- **KH2's Aerial Dodge on KH1's air state, no borrow.** The tester (aerial-dodge-native m1: "go ahead", plan item 1). The fresh jump in the air with Aerial Dodge: `combomods::aerial_dodge` asks our request of KH2's motion 202 (`kh1rows` "Aerial Dodge", request only, slot on KH1's air swing 102 with its sounds left out, 160 frames with KH2's loop baked in), state 1 put in state 2 as KH1's launch (Steam `0x1402B1960`) does, facing the stick, KH2's hop with the `fmab` height / speed of the level (KH2 `0x1403CB9F0` / `0x1403CBB00`), launch kept for KH2's glide; KH1's air step steers it as a jump. The borrow stays as the fallback (forms, slot not built). Checked (decomp) KH2's handler and KH1's loop at a slot's end; Assumption: 160 frames covers every rise. Notes `docs/kh1-aerial-dodge-native.md` §6.

### 2026-10-24: Stage 491 (testing branch: Sleeves thread: the coat moves with the chest as KH2's jacket)
- The tester (same answer). KH2 draws its jacket on torso bone 67 only, which turns as the chest part; its coat ring (129-135 / 221-227, 70) carries no mesh (Checked (files)). KH1's coat (72-75, 248-279) is carried rigidly by KH1's chest 78 as in the bind (`retarget::ATTACHED`), live and baked. Out: the pelvis turn, the front panels' thigh share and `keep_clear` on its nine chains (ours, s284-t489). Measured over 2954 frames: rigid to 0.08°. KH1's coat is longer than KH2's jacket, so its front tips reach 2.9-3.5 from a thigh's axis in kicks (Checked (files)); open question in the thread. Test `local_own`. docs/kh1-secondary-motion.md "The coat".

### 2026-10-24: Stage 490 (testing branch: Sleeves thread: shorts hang on KH1's thigh as KH2's)
- The tester (Missing sleeves animation, after m11): "make everything 1:1 with how its animated in kh2, if there's no equivalences like the ones we talked about then don't add them". KH2 draws its shorts on its pelvis 2 and on 24 / 53, which the thighs carry; its trouser bones (20, 22, 27, 29, 32, 34 and the other leg's) carry no mesh. 2 turns as the pelvis part, and 24 / 53 as the thigh part in the median frame, over all 83 motions (Checked (files): `P_EX100.mdlx` / `.mset`). `retarget::KH1_OWN` is empty: the shorts' panels (26-41 / 52-67) no longer take KH1's own animation (t387) and keep their bind relation to KH1's thigh, live and baked; the `set_own` machinery is kept with no bones. KH2's 9-30° key of 23 / 24 off the thigh joint (203-208, 42, 171) isn't carried. Test `local_own`. docs/kh1-secondary-motion.md "1:1 with what KH2's model shows".

### 2026-10-24: Stage 489 (testing branch: KH2's guard, its repel and Counterguard on KH1's states)
- The tester (m15 3 (a); 4 "(a) but read the mset"). Square standing with Guard (Dodge Roll KH2) is our KH1 request 0x208 for motion 173 (no borrow; KH1's action flag, idle queued, lock faced; `combomods::guard_press`); its Keyblade guard 161 (6..37, refact 1) stops hits from in front through `hurt` (`kh2slots::keyblade_guard`) and KH1 guard objects (`combat::guard_step`); the repel is request 0x209 / motion 42 faced to the attacker, pushed back 200 through `+0x120`; Counterguard is request 0x20A, row 39 / motion 171, ability 96, taken by KH1's Attack step in a combo window opened during the repel. Range id 2 (KH2 `0x14040D810` bit 0x10, Checked (decomp)) is `WindowKind::Free`, KH1's "may be left" on our requests. Guard Break: W_EX010 165's hit 876 is kind 1 (pierce armor) plus its own guard 162 at 5..42 (Checked (files)); KH1's priority rule kept (Assumption: kind 1 is KH2's guard break). Assumption: KH1's ground step moves him by `+0x2C` under the action flag. docs/kh1-anim-slots.md §7, docs/kh1-clash.md.

### 2026-10-24: Stage 488 (testing branch: KH2-only combo abilities make the combo KH2's)
- The tester (KH2's combo row selection, m15): "go with the recommended choice you had then" ((b): KH2's picker and rules) and "yes only the ones kh2 doesn't have". With Finishing Plus, Negative Combo or Berserk Charge (107) equipped, `styles::kh2(GroundCombo | AirCombo)` reads KH2 whatever the setting; `styles::setting` keeps the Config row showing the player's choice; `menu` feeds `styles::note_abilities` on each ability change (logged). Combo Plus / Air Combo Plus / Combo Master (KH1's own) switch nothing. Berserk Charge isn't in the ability list yet. docs/kh1-move-styles.md.

### 2026-10-24: Stage 487 (testing branch: KH2's stop at an air cast)
- The tester (m3 answer 2 (a), "with the exception to drive forms for now"). `shared::physics::cast_stops` (from `kh1_spell_start`, air state 2, base Sora, physics KH2): sideways speed 0 and KH1's arc reset to a fall from rest, then KH2's gravity and the cast's no-gravity hold (range 3). KH1's own air cast keeps 0.7 of the speed and the rise (`0x1402A3600`, Checked (decomp)). Assumption: KH1 doesn't move him on the ground while its action flag is set. docs §4d.

### 2026-10-24: Stage 486 (testing branch: KH2's casts as a step of KH1's combo)
- The tester (m3 answer 1 (a)). Cast slots get KH1's combo window (event type 5) from KH2's range 14 to the end (`kh2slots::cast_follow`; Fire 56 from 30, Blizzard 59 from 24, Thunder 62 / Cure 65 from 26, air 58/61/64/67 from 36/44/38/38, finishers 57/60/63 from 36/48/42; Checked (files) `P_EX100.mset`), finisher casts none; a fresh jump or Square there clears the action flag (`formmagic::cast_cut`); KH1's count `+0x41` and `+0x50` set at the native start (`formmagic::take_step`; KH1's Attack step `0x1402A7510`, `+0x50` cleared by `0x1402B3270`, Checked (decomp)); spells only from range 35 (`cast_magic_window`). Open: Finishing Plus after a finisher cast. docs/kh1-magic-native.md §4c.

### 2026-10-24: Stage 485 (testing branch: Config > Magic: Cure row)
- The tester (m3 answer 5): "make a menu in the magic config to change what gets casted kh1 cure or kh2". `styles::Kind::Cure`, ini `cure_style` (default kh2, as before), a Cure row in Config > KH1 in KH2 > Combat > Magic. KH1 = KH1's own Cure drawn on KH2's Cure cast motion and run on its clock, as Gravity / Stop / Aero at Magic KH2 (`formmagic::kh2_cure` / `kh2_spell`, `native()`, `spell_chosen`, `takeover::command_started`). Only with Magic KH2, not in Drive Forms. Trace column `cfg_cure`. docs/kh1-move-styles.md.

### 2026-10-24: Stage 484 (testing branch: KH2's MP costs on KH1's MP)
- The tester (KH2 magic and Reflect native, m3 answer 6 (b)). `mpcost.rs`: a KH2 spell costs (KH2 cost / 100) of Sora's KH1 max MP, from the top down (the charge part at the next bar's worth, bar k worth k x 30), paid through KH1's payment `0x1402A5B60` (Steam) 30 units a call; Cure takes all (at least one unit). Base Sora's KH2 spells and Reflect; Drive Forms keep 1 MP. KH1's usable check `0x14028EFD0` reads our cost (spell data `+0x30`, capped 99) for the one command step (`CostPatch`). Checked (files): Fire 12, Blizzard 15, Thunder 18, Cure 255, Reflect 10 (03system `cmd`), KH2 Sora's MP 100 (`plrp`); Checked (decomp) `0x1402A5B60`, `0x1402A4C10`, `0x1402A4CB0`, `0x14028EFD0`. Open: KH1's menu still greys by KH1's cost. docs/kh1-magic-native.md §6.

### 2026-10-24: Stage 483 (testing branch: Reflect's effects at KH1 Sora's size)
- The tester (m16, 2 (a)): scale Reflect to KH1 Sora. `kh2dpx::kh1_sized`: Reflect's packages' elements' scale and position-only offset times `pose::kh1_size()` (the retarget's hip ratio, about 0.89, new `KH1_SIZE`), at each element's start. KH1's instance scale carries the header offset, Checked (decomp) `0x14031CEA0`. The barrier's attack shapes (ATKP 573) are not scaled (gameplay). docs §18.22.

### 2026-10-24: Stage 482 (testing branch: KH2's object frame on KH1's object root, through KH1's apply turn)
- The tester (m16): Reflect "(b) off to one side"; Firaga "it isn't changed" (one flame). KH1's 0x8000 frame read straight (`0x14029EA00`) is (back, up, -x); through the apply (t481's P) it is (-x, down, back), KH2's object frame, so bound object-root elements keep KH2's offset and turn: t454's quarter turn `C` is gone (it turned them 90° about Sora). Position-only offsets now go through P; under `C` their height was reversed, putting Fire 3's ring (elements 2, 6, 7, 10) 72 to 218 below his feet. Checked (exe/trace/files: `FIRE_3.mag` effect 2, VertexApLc `0x14033D330` / KH2 `0x1404D3E00` the same). Also changes Explosion's burst, the forms' root effects, 0x4001 / 0x4009 elements. `note_place` prints KH2's offset along the actor's rows. Assumption: 0x8005 reads the same frame. docs §18.21 (§18.17 superseded).

### 2026-10-24: Stage 481 (testing branch: Blizzard's shard turned as KH2's)
- The tester (KH2 effects in KH1's own effect system, m16): "blizzard is sideways". KH1's apply `0x1402ADB40` (Steam) takes its angles with `0x1402A8EF0` (x = atan2(-row1.z, row0.z), y = asin(-row2.z), z = atan2(row2.y, row2.x)), which isn't the inverse of KH1's X-Y-Z build `0x140323480`, so an element given M is drawn M·P (P: columns z, -y, x; P·P = 1). Checked (exe); the t478 log's object-root turn agrees, Checked (trace). Blizzard 3's shard is placed by its script's `trap_effect_set_dir` (KH2 trap 0/41, table `0x140753040`, `0x14041E100` -> `0x1403B7710`: +z along the direction; Checked (exe/decomp); traced on `BLIZZARD_3.mag`, Checked (files)). `kh2dpx::spell_place` now passes `at·P` (`KH1_APPLY_TURN`); Wisdom's shots and Reflect's contact flashes go the same way. Assumption (open): bone- and Keyblade-bound pieces are also drawn B·P. docs/kh2-effects.md §18.20.

### 2026-10-24: Stage 480 (testing branch: Frame Advance: KH2's camera on the mod's clock)
- The tester (Frame advance in the debug menu), on t465's question about KH2's camera timing its gaps in real time: "unsure about your questions ... do the check ... go ahead". `camera.rs`'s `now_ms` (its 300 ms "was ours", 5 s "untouched" and 150 ms follow-call windows, the trace and jerk logs) is now `frameadv::clock_ms` (real time less the time Frame Advance held the game), so a hold over 5 s no longer starts KH2's camera over behind Sora on the next step; subtractions saturate. Select while held stays blocked (no change). Checked (code): the windows in `camera.rs`. Assumption: no window there wants real time across a hold. docs/kh1-frame-advance.md.

### 2026-10-24: Stage 479 (testing branch: Config menu: the glint follows the cursor)
- The tester (Config menu rework): "the glowing cursor that follows each command is not following properly, it stays at the top. use the c decomp". The glint is KH1's `0x1402D9B40` (a 30 x 30 glow at a point), its target from `0x1402D9920 (win, frames)`: the selected label's top-right corner, y = window y + 71 + row x window `+0xF0`. KH1's Config list def has a row pitch of 0 (`0x1405463A0 +0x22`), so in KH1 itself it stays on row 0. Our lists' defs now carry the rows' pitch (26 + 2), so KH1's own call follows the cursor; on page 1, after KH1's update, `0x1402D9860 (8, x, y)` (Steam) puts it at the selected row's corner taken through the 92% scale. Checked (decomp) for both functions and the window fields, Checked (exe) for the def's pitch of 0. Assumption: the glint's size stays unscaled on page 1. Sub-lists stay full size (answer 1a). docs/kh1-config-menu.md §5.

### 2026-10-24: Stage 478 (testing branch: KH2's finisher casts on KH1's combo)
- **KH2's finisher casts on KH1's combo.** The tester (same thread). 57/60/63 as alternates on 0x36..0x38 / 0x3D, `kh2slots::pick_cast`; KH2's rule on KH1's count (`+0x41`, `+0x50` or Combo Master, length `+0xD4`), finisher sub and voice. Notes `docs/kh1-magic-native.md` §4b.

### 2026-10-24: Stage 477 (testing branch: KH2's casts in KH1's own cast state)
- **KH2's casts in KH1's own cast state.** The tester (thread kh2-magic-and-reflect-native). `kh2slots::CASTS`: KH2 56/59/62/65 and air 58/61/64/67 on KH1's requests 0x36..0x39 / 0x83..0x86, plus 0x3D / 0x8A; no template leave point. `combat::command` doesn't borrow; hook on KH1's spell start `0x14028F140` (Steam, Checked (exe)) runs KH2's script instead (`formmagic::kh1_spell_start`): KH1's MP paid, MAGC voice, Fire's second part dropped from the queue; `takeover::native_spells` releases at the clip's magic-cast event on KH1's time. KH1's voice group off at Battle Quotes KH2. Notes `docs/kh1-magic-native.md`.

### 2026-10-24: Stage 476 (testing branch: Finishing Plus's next finisher on a KH1 row)
- **Finishing Plus's next finisher on a KH1 row.** The tester (m14 4 (b)). Rows / requests 0x206 / 0x207 with every finisher slot (`pick_own`); KH2's window (range 50) as KH1's combo window (instance `+0x14` bit 22, `0x1402A2EC0`); KH2's finisher count kept. Notes §7.

### 2026-10-24: Stage 475 (testing branch: Aerial Recovery and Retaliating Slash on KH1 requests)
- **Aerial Recovery and Retaliating Slash on KH1 requests.** The tester (m14 3 (b)). Requests 0x205 / 0x204; `combomods::recover` (KH2's hop 100 / 10), `Character::would_take_row`; KH2's free state as KH1's "may be left" on our requests. Assumption: KH1's knock-back ends with flag 4. Notes §7.

### 2026-10-24: Stage 474 (testing branch: KH2's Square moves and Guard Break on KH1 rows of their own)
- **KH2's Square moves and Guard Break on KH1 rows of their own.** The tester (m14 2 (b)). `kh1rows`: Sora's table (`0x142D5CAB0`, block `+0xA9F0`, 66 rows, Checked (exe)) + rows 66.., requests 0x200.., own data (launch kind; Guard Break priority 0x62, `0x1402B4BC0`, Assumption); `scale_record` KH2's numbers; Square through KH1's Attack command (`0x140293480`); air fall queued; `kh2slots::air_root` (Finishing Leap). Counterguard left. Notes §7.

### 2026-10-24: Stage 473 (testing branch: KH2's hits on his body on KH1's rows)
- **KH2's hits on his body on KH1's rows.** The tester (m14 1 (a)). Vicinity Break 170 / Explosion 166 / Magnet Burst 197 slots (`kh2slots::body_strike`); `takeover::kh2_hits` (`RigAnim::pose_at`, `Strikes::strike_as`, info `+0x50`); KH1's sphere on Keyblade windows only; range 27 in `hurt`. Notes §7.

### 2026-10-24: Stage 472 (testing branch: Debug menu: the font in its own buffer)
- **Debug menu: the font in its own buffer.** The tester ("if the debug allocates for kh2 model then make sure its gone"). `fx12::FontBuf` 147 KB; the 24 MB texture arena only made by a model draw. Notes §18.11.

### 2026-10-24: Stage 471 (testing branch: Debug menu: KH1 Spells)
- **Debug menu: KH1 Spells.** The tester (a toggle for the KH1 magics KH2 changed). No KH2 effect starts for Gravity / Stop / Aero (t463 log; KH2 motions 62 / 65 have no trigger, Checked (files)); at Magic KH2 KH1's effects play on KH2's cast cut at frame 68. Per spell "KH2 cast" (default) / "KH1 cast", base Sora, ini `kh1_cast_*`; its page is on the main page. `formmagic::kh1_cast`. Notes §18.19.

### 2026-10-23: Stage 470 (testing branch: KH2 effects: 0x4009 on KH1's body centre plus KH2's 20)
- **KH2 effects: 0x4009 on KH1's body centre plus KH2's 20.** The tester ("try (c)"). 0x8005 plus KH2's kind-9 point minus its kind-1 (Sora 105 − 85, Checked (files)); KH1's body-centre height logged once. Notes §18.16.

### 2026-10-23: Stage 469 (testing branch: KH2 effects: Reflect's frame re-examined; spell pieces' start places logged)
- **KH2 effects: Reflect's frame re-examined; spell pieces' start places logged.** The tester ("reflect still is misplaced"). t454's frame holds (KH2 decomp `+0x40`, the old player's diag(1,−1,−1), motion 71's barrier at the hands, Checked (decomp / exe / files)). `note_place` logs each spell piece's start place beside KH2's numbers. Notes §18.17.

### 2026-10-23: Stage 468 (testing branch: KH2 effects: a spell's pieces in one DPD per file DPD)
- **KH2 effects: a spell's pieces in one DPD per file DPD.** The tester (Firaga missing flames). KH1's 10 texture slots per frame (`0x1401EDB00`, Checked (exe)); t463 log: level-3 spells needed 21..37. One DPD per file DPD as KH2's; split per effect when over. Assumption: the cause of the missing flames. Notes §18.10.

### 2026-10-23: Stage 467 (testing branch: KH2 effects: MatrixLoop (216) as KH1's)
- **KH2 effects: MatrixLoop (216) as KH1's.** The tester's t463 log (Blizzard's shard). Calc-only in both (KH1 `0x14032EBF0`, KH2 `0x1404C4540`, bytes equal but call displacements, Checked (exe)); `KH1_SAME`. Notes §18.9.

### 2026-10-23: Stage 466 (testing branch: KH2 effects: KH2's per-palette HD images)
- **KH2 effects: KH2's per-palette HD images.** The tester's t463 log (Thunder 3: 0xd231..3, 0xe651..2). KH2 `0x1404AECD0` (Checked (decomp)): each record of a TBP at first + 1.. ; `kh2dpx::palette_textures` after KH1's `0x14031EC10`; the builder keeps KH2's mask bits for own records. Notes §18.12.

### 2026-10-23: Stage 465 (testing branch: Frame Advance: the Camera row (Free: the camera moves while held))
- **Frame Advance: the Camera row (Free: the camera moves while held).** The tester ("toggle the cam movement on or off"). KH1's pad step runs outside the task list; 100000 `0x1401E5D90` steps the camera mode (`0x140506CE0`) from the live stick, round the held Sora, writing only the camera's state; 110000 builds the view from it (Checked (decomp), table Checked (exe)). Free: 100000 runs held through `camera_live` with the frame's scale (0 otherwise since t463), no Select (first person clears the lock-on and fades Sora), no R3 in first person; KH1's first-person fade `0x1425380F8` holds it. KH2's camera goes along (same scale). Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 464 (testing branch: Frame Advance: its rows on a debug menu page of their own)
- **Frame Advance: its rows on a debug menu page of their own.** The tester ("make frame advance a submenu and throw it plus frame advance option in there"). Main page: Frame Advance `>`; its page: Frame Advance, Held Sound, Game Frames (also on the Trace Recorder's page), Held Tasks. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 463 (testing branch: Frame Advance: every update held, every draw run)
- **Frame Advance: every update held, every draw run.** The tester ("focus on getting the live version working"). Runs while held: 10499, 110000, 129999, 135000 (Checked (decomp)); held: 163001, 190000 `0x1EB750`; frame scale `0x14233FBDC` and half `0x14233FBE0` 0 on held frames; 110000's zoom, 164004's pulses, the HUD data `0x1427F13D0`..`0x142866D18` and model colours put back each held frame. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 462 (testing branch: Frame Advance: the main list's tasks read one by one)
- **Frame Advance: the main list's tasks read one by one.** The tester ("look through the c decomp… do that for the rest of the values"). Every registration on the main list read (Steam decomp); 110000 is the view set-up (texture scroll, fog packet), not a camera; KH1's render queue `0x1401CB730` found. Held Tasks labels corrected. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 461 (testing branch: Frame Advance: Live only)
- **Frame Advance: Live only.** The tester ("the actual pause seems to hold the image in the frame buffer"). KH1's pause picture (task 190020, capture `0x14018B370`) and the Held Picture row removed; effects held by KH1's system stop as in t447. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 460 (testing branch: KH2's ability moves on KH1's rows at the combo style KH1)
- **KH2's ability moves on KH1's rows at the combo style KH1.** `native_ability` style-free, `Kh1Combo::here`, `kh2slots::pick_ability`. Notes §7.

### 2026-10-23: Stage 459 (testing branch: KH2's Combo Plus / Negative Combo picks on KH1's plain rows)
- **KH2's Combo Plus / Negative Combo picks on KH1's plain rows.** `Kh1Combo::serves`; no borrow at Base / Aerial Combo KH2. KH1's finisher is known by its request. Notes §7.

### 2026-10-23: Stage 458 (testing branch: KH2's turn toward the target in a swing on KH1's rows)
- **KH2's turn toward the target in a swing on KH1's rows.** Range 28 (`0x1403D4B10` / `0x1403D3A90` / `0x1403B9F70`, Checked (decomp)): up to 2 × the parameter an update toward `+0xDC8`, past 50 away. `kh2slots::turn_to_lock` from `takeover::physics`; target `takeover::kh2_primary_target`. Notes §7.

### 2026-10-23: Stage 457 (testing branch: KH2's edge stop on KH1's ground swings)
- **KH2's edge stop on KH1's ground swings.** Range id 29 (`0x14040D810` → `+0x18C` bit 23, Checked (decomp)): `0x1403B9090` sweeps the step + 0.2 (`0x14074B060`, Checked (exe)) × 90 and keeps only its vertical part. `kh2slots::edge_stop`, `shared::physics` ground step, `combomods::floor_gone`. 154 / 155 / 165 / 169 / 161 (Checked (files)). Plain swings still cut into the fall, as in KH2 (`0x1403D6430`) and KH1 (`0x14029A3C0`). Notes §7.

### 2026-10-23: Stage 456 (testing branch: fx12's vertex ring sized for the debug overlay)
- **fx12's vertex ring sized for the debug overlay.** The tester (3 (a)). 4 x 1 MB (was 4 x 6 MB, sized for the old effect player): one overlay draw is at most 18 lines and 2 boxes. The KH2 player model (off) would need it sized again. Notes §18.11.

### 2026-10-23: Stage 455 (testing branch: KH2 effects: KH2's flag 0x800 (alpha follows the owner))
- **KH2 effects: KH2's flag 0x800 (alpha follows the owner).** The tester (2 (a)). KH2 `0x1402C58F0` / `0x1402C6A10` (Checked (decomp)): while bound, the effect's alpha is the weapon's (0x4003..8) or owner's, 0 hidden. KH1: Sora's draw bit and `+0x60`, the weapon-out flag for the Keyblade; fade-out on its own level. Assumption: KH1's Keyblade alpha byte unchanged in play. Notes §18.18.

### 2026-10-23: Stage 454 (testing branch: KH2 effects: KH2's object frame on KH1's object root)
- **KH2 effects: KH2's object frame on KH1's object root.** The tester ("reflect is misaligned with sora"). KH1's 0x8000 frame (`0x14029EA00`: rows -+0xF0, -+0xE0, -+0xD0 of the skeleton pass's matrix, Checked (exe)) is KH2's object frame (x, -y, -z: the old player's, Assumption) a quarter turn about y; offsets and turns on 0x8000 / 0x8005 go through `KH2_IN_KH1_OBJECT` (copy, position read, script place). Notes §18.17.

### 2026-10-23: Stage 453 (testing branch: the coat as before t387)
- **The coat as before t387.** The tester (after t412): "before I prompted you to adjust the bake animations the coat was working how it should", "model ... also kh2 sora's skeleton". t387 put the jacket hem (73-74, 249-278) in `KH1_OWN`: it took KH1's own motion's relation (baked: the template; live: KH1's pass, hence t405 / t411). Out again: `KH1_OWN` 34 -> 16 (the shorts); the coat is s284..t386's (roots on KH2's pelvis, front panels a thigh share, joints at bind, `keep_clear`), live and baked. Checked (files): all KH2 motions (11906 frames) with KH1's run fed, the coat's 36 bones identical to t386's (t412: up to 27.7). KH2's coat ring (129-135 / 221-227, 70, from chest 68) carries no mesh. Test `local_own`. Notes `docs/kh1-secondary-motion.md` "The coat as before".

### 2026-10-23: Stage 452 (testing branch: KH2's first-person pitch rate and limits from KH2's exe)
- **KH2's first-person pitch rate and limits from KH2's exe.** The tester ("check the decomp"; the values are exe data). Read from the tester's Steam KH2 exe (SHA-256 `9002b2de...49ed`; its `0x140165e30` reads them, Checked (exe)): `0x140718bf4` 3° a frame, `0x140718bf8` / `0x140718bfc` ±70°. `kh2_camera::FP_PITCH_*` (was KH1's 80° up / 60° down). `0x140718bf0` (the turn) is 3° too, as ours. Notes `camera-model.md`, `kh2-pad.md`.

### 2026-10-23: Stage 451 (testing branch: KH2's camera springs once a frame at 30 fps)
- **KH2's camera springs once a frame at 30 fps.** The tester ("(a)"). `kh2_camera::settle`: one spring a frame, the rate halved when the step is under 2, whole from 2 (Steam KH2 `0x140165700`, Checked (decomp)); was two half springs. Keyboard / mouse at 30 fps closes about 91% of the gap a frame (was 75%); pad 17.5% (was about 16.7%). Notes `camera-model.md`.

### 2026-10-23: Stage 450 (testing branch: The mouse turns Sora in KH2's first person)
- **The mouse turns Sora in KH2's first person.** The tester ("check the decomp"). KH2 `0x1403a9250`: ±3° a frame on the direction bits, set from the mouse in a mouse frame (`0x14039c180`). `camera::first_person_mouse_x` (KH1 `0x140138b30`, right stick `0x14298cac0`); forms `first_person_turn`; base Sora: KH1's turn `0x14028c7e0` fed a full left stick `0x14298cad0` for its control step `0x1402a6220`, put back after. Steam only. Assumption: bits 20 / 21 are the right stick's x. Notes `docs/kh2-pad.md`.

### 2026-10-23: Stage 449 (testing branch: Config menu: KH1 in KH2)
- **Config menu: KH1 in KH2.** The tester (answers 1 Magic inside Combat, 2a, 4b, 5a, 6b). Page 1 has 9 rows: the button above Default; page 2 gone. Lists are KH1 windows (`0x1402E5A40` / `0x1402E3D10` / `0x1402E5E40`, KH1's help line). Back-out goes through the title tab `0x1402FADB0` (prologue `40 57 48 83 EC 40`, Checked (exe)). Steam only. Notes §6.

### 2026-10-23: Stage 448 (testing branch: Config list at 92% with KH1's window scale)
- **Config list at 92% with KH1's window scale.** The tester (answer 3a). Window `+0x6C` bit 0, `+0xA0`/`+0xA4` 0.92, anchor `+0xD8` 1, set in `0x1402FA210` (Checked (decomp)). Notes `docs/kh1-config-menu.md` §5.

### 2026-10-23: Stage 447 (testing branch: Frame Advance's Held Picture row: Live)
- **Frame Advance's Held Picture row: Live.** The t426 split with effects stopped by KH1's system stop (`0x142867368` bit 0, `0x142432130`, Checked (decomp) and (exe)), KeShpTail3 / 3X draw steps taken back, and 120000's sky / fades restored. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 446 (testing branch: Frame Advance holds the sound as KH1's pause does)
- **Frame Advance holds the sound as KH1's pause does.** `0x1400D2F80` (1 / 0) at hold start / end (pause states 3 and 4, Checked (exe)). Held Sound row.

### 2026-10-23: Stage 445 (testing branch: Frame Advance holds what KH1's pause holds and shows KH1's pause picture)
- **Frame Advance holds what KH1's pause holds and shows KH1's pause picture.** KH1's pause: mask 1, task 190020 (`0x14017FD60`), snapshot `0x14018B370`, redraw `0x140183CD0` (Checked (exe)). Every task without the pause flag held; our task at 190020 snapshots once per hold and redraws with spread 0. `0x1401ED6D0` also sets `+0xDB`, which hides effects (Checked (decomp)). Assumption: the snapshot sees the last frame. Held Tasks rows labelled. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 444 (testing branch: KH2 effects: the old player deprecated)
- **KH2 effects: the old player deprecated.** The tester ("the older player is to be deprecated"). `kh2fx` is the entry point to `kh2dpx`; player, `fx12::draw` / effect textures and `fx_test` removed; world-pass hook in `model` (installed only while the model is on); misses logged once. t430 log: the old player held 176 textures, 16,704 KB. Notes §18.11.

### 2026-10-23: Stage 443 (testing branch: KH2 effects: a form's package built once Sora could change into it)
- **KH2 effects: a form's package built once Sora could change into it.** `form::possible` (MP, data, Valor's Keyblade read without writing). Notes §18.12.

### 2026-10-23: Stage 442 (testing branch: KH2 effects: unbound position-only elements through KH1)
- **KH2 effects: unbound position-only elements through KH1.** Mode-1 start, place set from `by_position_now`, disarmed. Checked (exe). Notes §18.7.

### 2026-10-23: Stage 441 (testing branch: KH2 effects: logging HD images a texture record didn't find)
- **KH2 effects: logging HD images a texture record didn't find.** t430: Thunder 3's TBPs 0x3000/0x3020 untextured. KH2's extra per-record case (`0x1404AECD0`, 0xFFFD, Checked (decomp)) noted; not fixed. Notes §18.12.

### 2026-10-23: Stage 440 (testing branch: KH2 effects: only the elements that play are built)
- **KH2 effects: only the elements that play are built.** The Keyblade's package: its vanish flash (t430: 4 effects, 616,048 bytes). Notes §18.7.

### 2026-10-23: Stage 439 (testing branch: KH2 effects: KH2's 0x4009 point)
- **KH2 effects: KH2's 0x4009 point.** t430 log (Cure 3: 3, 4). KH2 = object turn at the first enabled kind-9 shape's centre (`0x1403B50F0`, Checked (decomp); kind 9 is read nowhere else); Sora's lift 105 on the root (Checked (files)). KH1 0x8000 with the offset moved (`effect_point_of`). Notes §18.16.

### 2026-10-23: Stage 438 (testing branch: KH2 effects: the spells' second-weapon points)
- **KH2 effects: the spells' second-weapon points.** t430 log (Fire 3: 9, 12; Blizzard 3: 7). KH2 0x4004/6/8 = owner weapon slot 1 for any package (`0x1403B5450` → `+0xD68`, Checked (decomp)); `spell_anchor` = `form_anchor`. Assumption: started only with two weapons. Notes §18.10.

### 2026-10-23: Stage 437 (testing branch: KH2 effects: position-only on KH2's object root)
- **KH2 effects: position-only on KH2's object root.** t430 log (Fire 3: 2, 6, 7, 10). KH1 0x8000 point at t 0 = actor `+0x10` (`0x14029EB20` last case, Checked (exe)); `posed` offset through the object frame. Notes §18.5.

### 2026-10-23: Stage 436 (testing branch: KH2 guard on his body in KH1's states; Flash Step)
- **KH2's guard on his body in KH1's states; Flash Step.** `kh2slots::body_guard` (ATKP `refact_self` 2, 00battle Checked (files)); `hurt` marks KH1's records no-effect with KH1's clang. Flash Step 169 native. Notes §7.

### 2026-10-23: Stage 435 (testing branch: KH2 Aerial Spiral and Aerial Finish on KH1's air combo)
- **KH2's Aerial Spiral and Aerial Finish on KH1's air combo.** Air ability rows through `Kh1Combo::plain` / `air_hop`, 192 on 0xCC / 0xCD, 194 on 0xCE. Notes §7.

### 2026-10-23: Stage 434 (testing branch: KH2 no-gravity ranges on KH1's air state; Aerial Dive)
- **KH2's no-gravity ranges on KH1's air state; Aerial Dive.** `kh2slots::no_gravity`; `shared::physics` air step holds the arc (KH1's step undone / KH2's arc not advanced). Assumption: the exe matches the OG rewrite. Aerial Dive 196 (46..54) native. Notes §7.

### 2026-10-23: Stage 433 (testing branch: KH2 Aerial Sweep from the ground natively)
- **KH2's Aerial Sweep from the ground natively.** Row 33 (191) through `kh2_hop`, as row 0, slots on 0xCC / 0xCD. Notes §7.

### 2026-10-23: Stage 432 (testing branch: KH2 Keyblade-only ability swings on KH1's ground combo)
- **KH2's Keyblade-only ability swings on KH1's ground combo.** `kh2slots::blade_swing` (weapon hits only, no body attack, guard, invulnerability or air/ground range; P_EX100.mset, Checked (files)); `combomods::native_ability`: Slapshot 162, Dodge Slash 163, Slide Dash 164 on 0xC8..0xCA, no borrow (root travel 50 / 300 / 700). 9 slots, about 1.5 MB. Notes §7.

### 2026-10-23: Stage 431 (testing branch: KH2 ledge hop checked in the t430 traces)
- **KH2's ledge hop checked in the t430 traces.** f48640 / f48888: row 1 about 40 from the edge, 21 high, 7.8/frame off it (t415 holds). f48311 / f49221: row 6 about 89 out (KH2's rule), 153's travel took him off. KH2's ground-mover edge stop (`0x1403B9090` / `0x1403BAB20`, decomp) is gated on `+0x18C` bit 23 (setter not found; Assumption: no swing sets it). No behaviour change. Notes §7.

### 2026-10-23: Stage 430 (testing branch: KH2 effects: KH2's body centre 0x4001 on KH1's 0x8005)
- The tester (option (a)). KH1 0x8005 (the first enabled kind-1 collision part's centre, object frame), Checked (exe);
  KH2's 0x4001 the same in KH2 (decomp); all packages; both heights logged once (KH2's: 85 for Sora). Notes §18.15.

### 2026-10-23: Stage 429 (testing branch: KH2 effects: KH2's pppSnoMdlDttNorm, the Drive sphere through KH1)
- The tester (option (a)). `kh2ppp::dtt_draw` (KH2 `0x1404CBBA0` / `0x1404CAD20`, Checked (decomp)) with KH1's pppKeMdlDtt calls;
  construct and reset KH1 145's. Assumptions: normal 1/4096, push x strength. Drive element 1 through KH1. Notes §18.14.

### 2026-10-23: Stage 428 (testing branch: KH2 effects: the forms' Keyblade points on KH1's blade binding)
- The tester (option (a)): 0x4003 / 5 / 7 KH1 blade binding on Sora; 0x4004 / 6 / 8 (KH2's second weapon) on
  `leftblade::left_owner` (refreshed after Sora's pass while bound; stopped when it goes, assumption). Notes §18.8, §18.13.

### 2026-10-23: Stage 427 (testing branch: KH2 effects: packages built when needed, lean, freed when not)
- The tester ("can we have the files generated at run time so we don't take up extra memory?"). `kh2dpx::manage`: wanted
  packages (Sora's for the `kh2_effects` moves + Drive and Revert, the Keyblade's, a form's from the Drive command to revert,
  spells and Reflect at Sora's levels) built in the background, entered at once, freed by KH1's removal `0x1400E2E00` once
  idle. One 32 MB region for all DPXs; one HD entry per DPD / TBP (`0x14031EC10`, Checked (exe)); common images one KH1
  texture each from `kh2fx_texcommon_dpx` (`0x14010C050`), shared. Sora's: 6.2 MB + 4.3 MB held -> 1.7 MB, none held. Until a
  package is built `kh2fx` plays its effects. Notes `docs/kh2-effects.md` §18.12.

### 2026-10-23: Stage 426 (testing branch: Frame Advance draws the level and the models while held)
- The tester's Held Tasks test (t399): 130000 (`0x2A23E0`) draws the models, 120000 (`0xD2830`) the level, and with 120000
  running characters can move; 100000 (`0x1E5D90`) seems to be the camera. `frameadv`: 120000 and 130000 run while held by
  default (`RUN_BY_DEFAULT`); 120000's own task lists (`0x14233E420` / `0x14233E7D0`: the animated map parts) are held
  instead, each a row on the Held Tasks page (`SUB_LISTS`). Assumption: those are what moved. Notes:
  `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 425 (testing branch: KH2 effects: the spell scripts' effects through KH1)
- Reflect, Wisdom and KH2 magic packages, Steam only: the old player's spell functions (start, place, end, kill, alpha,
  running) try KH1 first (`NATIVE_SPELL`); place through KH1's apply `0x1402ADB40` (Checked (exe)). What remains on `kh2fx`:
  notes §18.10-18.11.

### 2026-10-23: Stage 424 (testing branch: KH2 effects: KH2's pppAngleLoop and pppKeShpTail3X)
- `al_calc` (KH2 wraps angles above 0x10000) and `tail3x_draw` (KH1's draw, then the trail step taken back when held, paused
  or on a computed frame), Checked (decomp). Seven more programs added to `KH1_SAME` (SRandFV, SRandUpFV, KeMatSN,
  KeShpTail2X, PointLoop, ScaleLoop, DrawMdlLoop). Notes §18.9.

### 2026-10-23: Stage 423 (testing branch: KH2 effects: Valor's and Final's packages through KH1)
- Body and object-root anchors through KH1; weapon points stay on `kh2fx` (open question). Notes §18.8.

### 2026-10-23: Stage 422 (testing branch: KH2 effects: one DPX per package; the Keyblade's vanish flash and unbound elements)
- Unbound elements: KH1's start, then the binding disarmed (mode 0 + bind-once, Checked (exe)). Vanish flash at blade
  t = 0.5 (assumption, as the old player). `plug` flags only ever turn on. Notes §18.7.

### 2026-10-23: Stage 421 (testing branch: KH2 effects: the Drive motion's and Revert's flashes through KH1)
- Drive 2..13 and Revert 15..27 through KH1. Element 1 (KH2-only pppSnoMdlDttNorm, 238) stays on `kh2fx`. Notes §18.6.

### 2026-10-23: Stage 420 (testing branch: KH2 effects: position-only elements on KH1's binding mode 1; the rest of Sora's moves through KH1)
- KH2's flag 0x04 elements on KH1's binding mode 1 (start `0x1402ADC20`, Checked (exe)); after Sora's pass `posed` sets the
  record's offset to bone place + turn x offset - KH1's point. Guard Break, Finishing Leap, Aerial Recovery, 169 / 171 / 172.
  Notes §18.5.

### 2026-10-23: Stage 419 (testing branch: KH2 effects: KH2's pppVtMime calc; Vicinity Break and Aerial Finish through KH1)
- `kh2ppp::vt_calc`: KH2 skips a rate key below -1e38, and Sora's keys use it (KH1's would wreck the morph). KH1's draw,
  construct, reset and destruct kept. Assumption: KH2's calc from the §7.4 reading (not in the Steam decomp). Notes §18.4.

### 2026-10-23: Stage 418 (testing branch: KH2 effects: Sora's whole package built for KH1; Magnet Burst through KH1)
- Groups per motion (moveset frame triggers), Revert's set, then one per effect. Bind-pose bone table (§18.3). Gate:
  `KH1_SAME` programs (decomp comparison §18.2) or KH2's own plugged in; anything else goes to `kh2fx`, logged once.

### 2026-10-23: Stage 417 (testing branch: KH2 effects: the KH1 DPX builder for any effects)
- The tester ("KH2 effects in KH1's own effect system": "port every effect"). `kh1dpx::build_groups`: one DPD per group,
  from any DPDs of a PAX; each entry's program types and each texture's source DPD reported; a group over KH1's 10 / 6
  texture slots is left out and logged. Explosion's DPX unchanged (byte-identical offline). Notes `docs/kh2-effects.md` §18.1.

### 2026-10-23: Stage 416 (testing branch: KH2's line ahead, its floor rule and direction)
- The t409 traces: row 1 at a 120 drop, 50..70 from the edge, is KH2's. The test was KH1's (`0x1402C6E70`: any step more than
  half KH1's height down) along KH1's press direction. Now KH2's (`0x140172640` / `0x140172840`, decomp): only the end of the
  85 line (`attack_j_front` -5 + `+0x6F0` 90; `+0x150` = plyr, Checked), none or more than lift - height - 1 = 24 below his
  feet (lowered by a downhill line), laid on the floor's plane (KH1's `0x1401907F0`), along `Character::selector_aim`
  (`0x1404028D0`). KH1's set-up / mover (`0x14029A0B0` / `0x140195180`, bytes checked) move it. Assumption: flat on
  objects. Notes `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 415 (testing branch: KH2's hop out of the ground combo keeps its speed)
- Mr Bugs (t409 traces f3990..f5413: "next to a ledge will start a short jump attack"): every row-1 hop rose and fell in
  place, `+0x3C0` 0 from its first frame. KH1's control step (`0x1402A3300`, decomp) sets `+0x3C0` = `+0x20` x `+0x2C` (the
  stick, from `0x1402AFC20`) on a frame its state changed into 2. `shared::physics::ground_hop_carry` puts `kh2_hop`'s
  velocity back before KH1's physics step. The hop firing there is KH2's (a 120 drop 50..70 ahead). Notes
  `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 414 (testing branch: KH2's mouse camera)
- The tester ("(b)"): KH1 PC's mouse code is KH2 PC's (Steam KH1 `0x140138b30` = KH2 `0x1404e51f0`). Mouse frames: both
  sticks KH1's processed ones times `1 + 0.02 s`; the follow camera turns `max(|10x|,1)·3°` and zooms `8·max(|10y|,1)`
  (`0x140166100`); keyboard/mouse mode (`0x1422DB3E8`) lag 0.4 + (run-8)·0.0018 (`0x140165700`). Steam only; prologue
  checked. Trace `kh2cam_mouse_x/y`, `kh2cam_kbm`. KH1's own mouse camera (`0x1401e7320`) compared in `docs/kh2-pad.md`.

### 2026-10-23: Stage 413 (testing branch: R2 holds KH2's camera swing)
- The tester ("its R2"): bit 32 of KH2's held word is R2. `kh2_camera::Controls::hold`: the swing behind runs only when
  `auto != hold` (Steam KH2 `0x140178ae0` / `0x140165110`). `camera::r2_held`: Steam KH1's raw pad word `0x14298C9B4`
  bit `0x200` (keyboard too; assumption), else `RAW_BUTTONS`; Frame Advance's R2 stays hidden. Trace `kh2cam_hold`.
  Notes: `docs/kh2-pad.md`, `camera-model.md`.

### 2026-10-23: Stage 412 (testing branch: KH2's hood is on its torso)
- The tester: "the hoodie doesn't seem to be animated anymore". From the files: KH2's model has no hood standing off the
  back; the mesh behind the neck on 140 is the hair's, the hood is the top of the back on the torso 67 (68 above carries
  none, no motion turns it off 67). `KH2_SHOWN_ON` hood 72 / 73 -> 67 (`KH2_TORSO`): KH1's hood keeps its bind relation
  to the neck base (t407 had it turning with KH2's head). Assumption: that patch is the hood. Notes
  `docs/kh1-secondary-motion.md` "KH2's hood".

### 2026-10-23: Stage 411 (testing branch: KH1's own parts: a skipped bone holds our matrix moved)
- Mr Bugs (t409 picture, trace f20377): "his coat, his pants ... return to the map origin". Both of KH1's passes end by
  adding the world translation to every bone (after the world scale, if off 1), placed or not (`0x140392560`,
  `0x140391C70`, decomp); t405's `Sora::wrote` test took the blend's skipped parents as placed, so the hem and shorts
  were measured one world position off on every blend frame (each stop from walk to idle and the frames after).
  `pose::kept`. Assumption: the scale's side. Notes `docs/kh1-anim-slots.md` §1.6.

### 2026-10-23: Stage 410 (testing branch: KH2's combo rows checked in the tester's traces)
- His t409 traces f37236..f44266 (physics KH1, Base / Aerial Combo KH2): every air pick and per-row hop is KH2's rule
  (heights 21 / 17 / 21, 67..71 toward an enemy above; speeds at the row bounds or the gap's). Notes and two comments
  corrected: KH2's row 10 and KH1's air row 17 are for a target above, 184 / 152 / 154 only off the primary's line. No
  behaviour change. Notes `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 409 (testing branch: KH2's walk / run split)
- The tester ("yes"). Steam KH2 `0x1403a8fa0` / `0x1403aa700`: walk below tilt 0.8, run from it, no dead zone past the
  pad's. `run_threshold` 0.8 (was 0.5), `DEAD_ZONE` 0 (was 0.1); the ledge hang keeps 0.1 (`HANG_PUSH`). Notes:
  `docs/kh2-pad.md`.

### 2026-10-23: Stage 408 (testing branch: KH2's first person in KH1's first-person mode)
- The tester ("KH2 Camera fixes", option (a)): Select's first person is KH2's. KH1's mode 2 entries swapped
  (`first_person_mode` / `first_person_exit`, Steam `0x1401E2390` / `0x1401E2610`; Epic from the table's neighbours) only
  if both are KH1's own. Steam KH2 `0x140165e30`: eye at the position + `prty` subjective_offset, 350 ahead along the
  facing, right stick up/down pitch, R3 levels, Select out. Sora fades over 30 frames each way through KH1's alpha
  `+0x60` / `+0x64`. He stands (`move_tilt` 0) and turns 3 deg a frame. Assumptions: the pitch rate and limits (KH1's) and
  which stick turns him. Notes: `camera-model.md`, `kh1-kh2-camera.md` 9c.

### 2026-10-23: Stage 407 (testing branch: only what KH2's model shows)
- The tester: "(b) only do what kh2 does so its 1:1". `secondary::KH2_SHOWN_ON`: KH2's keyed bones with no mesh read as
  held to their mesh bone (hood 72 / 73 and spikes 143 / 159 / 161 on 140, flaps 113 / 115 on 110, 205 / 207 on 202). Hood
  with KH2's head, sleeves without flap motion or squash; t386's scale pass now carries none of KH2's (files).

### 2026-10-23: Stage 406 (testing branch: the belt chain, belt-front pieces and pendant set aside)
- The tester: "remove them from animation for now", keeping the hem and shorts ("2. (a)"). `KH1_OWN` 41 -> 34 bones; 5, 8,
  11-14, 87 hang as before t387, live and baked.

### 2026-10-23: Stage 405 (testing branch: KH1's own parts on KH1's blend pass)
- Mr Bugs ("Missing sleeves animation"): "standing and returning to neutral cause parts of soras model to stretch like
  crazy" (his trace f8434). KH1's blend pass (`0x140392560`, `pose_ik`) writes a non-IK bone only with bone-table bit 30
  (decomp); every `KH1_OWN` part's parent lacks it (files), so `own_from_kh1` measured the shorts, hem and belt pieces
  against our last drawing. `Sora::wrote` tells a bone KH1 didn't place; it is rebuilt from the one above by KH1's last
  relation (constants only, files), else the part keeps its last. Notes `docs/kh1-anim-slots.md` §1.6.

### 2026-10-23: Stage 404 (testing branch: KH2 effects: the old effect player retired for Explosion)
- The tester (option (b)): on Steam with the DPX built and KH2's programs plugged in, Explosion is KH1's alone (no `kh2fx`
  fallback); `kh2fx` keeps it only before the DPX is built, if plugging fails, or off Steam. Ini `kh2_fx_native` removed.
  What still uses `kh2fx`: notes §17.8.

### 2026-10-23: Stage 403 (testing branch: KH2 effects: bound elements take their bone on KH2's frame)
- The tester (option (a)). From the decomps: both bindings apply the matrix read on their last run (KH2 `0x1402C58F0`, KH1
  `0x1402AE910`), but KH2's first run reads first, and it reads while waiting; KH1's start reads before Sora's skeleton
  pass. `kh2dpx::posed` (from `pose`, Sora only) re-reads our continuous bindings' records with KH1's `0x14029EA00` /
  `0x14029EB20`: after the pass on the start frame, before it while waiting. KH1's own bindings untouched. Notes §17.7.

### 2026-10-23: Stage 402 (testing branch: KH2 effects: KH2's pppColor for KH2's effects only)
- The tester (option (a), KH1's effects unchanged): KH2's pppColor (Steam KH2 `0x1404C0F60`, bytes written every call,
  clamped at 32767) as our third entry (`kh2ppp::col_calc`); `kh1dpx` lists our DPX's type-43 words and `kh2ppp::plug`
  points only those at it (KH1's entry 43 checked, `0x14032B940`). Assumption: KH2's scale 1/128 (KH1's `0x1403B30B8`);
  KH1's construct `0x14032BAE0`. Notes `docs/kh2-effects.md` §17.6.

### 2026-10-23: Stage 401 (testing branch: KH2's per-row hop on KH1's air swings)
- The tester: "(a)", 1:1. KH2 starts each row's hop when it is used, in the air too (`0x1403D78D0`, the count kept).
  Aerial Combo KH2, KH2's plain air row playing on KH1's 0xCC / 0xCD / 0xCE: `combomods::air_hop` / `Run::AirHop`;
  `combat::exec` runs KH1's air row, then `shared::physics::kh2_hop` puts KH2's height and speed on KH1's air fields in
  place of the row's own lunge (8 / 12 / 4) and hop. No slot: KH1's own. A press in KH1's take-off frames (state 1)
  keeps KH1's row's hop. Notes §7.

### 2026-10-23: Stage 400 (testing branch: KH2's attack hop rule from the KH2 exe)
- t392's hop numbers were the OG rewrite's; Steam KH2 `0x1403D78D0` (Checked, decomp): the primary target, from his own
  kind-3 target shape's centre (Sora's: 70 up, radius 55, `P_EX100.mdlx`) to the target's (`0x1403D7520`); height
  bounded by PTYA `+0x14` / `+0x10` (+ jump height with flag 2); speed (gap less both radii + `distance_adjust`, not
  held at 0) over `sqrt(|2h / 0.40816328|)` (`0x1403B73F0`), bounded by `+0x18` / `+0x1C`, the exe's test order.
  `AttackParams::target_lift` / `target_radius` (`kh2_data::sora::target_shape`). Assumption: which player class is
  Sora's (several share the method pair). Notes `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 399 (testing branch: Frame Advance's Held Tasks page)
- The tester's t378 test: the picture is still black while held; he asked how to help log it. His t393 log lists the 22
  tasks held (all under priority 140000). `frameadv::tasks` / `toggle_task`: the debug menu's new Held Tasks page (main
  page) lists them (priority, function RVA, Held / Runs); one switched to Runs runs while the game is held (this session),
  each switch logged with the list running. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 398 (testing branch: KH2 effects: Explosion's burst (element 48) through KH1)
- KH2 0x4000 to KH1's object root 0x8000; flag 0x02 (bound until started) to KH1's bind-once bit 0x10000. Needs both
  programs plugged in. Assumption: KH1's 0x8000 frame is KH2's object frame. All of Explosion now plays through KH1's own
  effect system on Steam. Notes §17.5.

### 2026-10-23: Stage 397 (testing branch: KH2 effects: KH2's pppEiDrawShipoly through KH1's renderer)
- Step 4. The ribbon ported (Steam KH2 `0x1404B9C00` / `0x1404B9260` / `0x1404B8380`: construct, reset, destruct, calc,
  the spline, the draw), drawn with KH1's GS setters, shader 0x2B, camera `0x14031EAA0`, vertices `0x1400F3040` and draw
  `0x1400FB9C0`; the spline matches `kh2-fx`'s to 4e-6. Not ported: the scroll-draw variant, spaces 2 and 3. Notes §17.3.

### 2026-10-23: Stage 396 (testing branch: KH2 effects: Explosion's elements 44 and 45 through KH1)
- Anchors from KH2 bone codes: 0x4003 / 5 / 7 to KH1's blade point (code 0x30, t 1 / 0 / 0.5, binding mode 3 `0x1402ADD60`,
  t the 5th argument, a float on the stack, Checked (exe)); KH2 67 to KH1 70 (same bind frame). They play only once
  KH2's MegaBirth is plugged in, else through `kh2fx`. Checked (files): KH2 170 / 78 and KH1 93 / 198 match within 0.05,
  so the wrist flashes need no turn conversion. Not done: flag 0x800 (alpha with the weapon). Notes §17.4.

### 2026-10-23: Stage 395 (testing branch: KH2 effects: KH2's pppRyjMegaBirth in KH1's program machine)
- Step 3 (the tester: option (a), port it). `kh1dpx` lists KH2-own program words (`Kh1Dpx::programs`); `kh2ppp::plug`
  points them at our 0x50-byte entry after KH1's relocation (`0x14031C2F0`), after checking KH1's entry 201. It is KH2's
  calc, step, birth, update and draw (Steam KH2 `0x1404BC3B0`..`0x1404BC710`) on KH1's instance, drawn through KH1's
  shape draw `0x14031E210`, with KH1's construct and destruct. Steam only. Assumptions: two operands the KH2 decomp
  drops (spin as turn-then-scale; `+0x1B0` the root's axis lengths). Notes §17.1-17.2.

### 2026-10-23: Stage 394 (testing branch: KH2 effects: bound elements start on KH2's frame)
- The tester Mr Bugs (`docs/tester-requests.md`, "KH2 effects in KH1's own effect system": "exact to kh2"): `kh2dpx::apply`
  takes KH1's 2-frame extra wait for bound elements (wait <= -2 x `0x14233FBCC`, `0x1401ECB50` / `0x1401ECF30`) off the
  wait it writes; both games test the wait before counting it down, so the start frame is KH2's. KH1's binding task
  (`0x1402AEDA0`, 139999) runs before the effect update (140000). Notes: `docs/kh2-effects.md` §16.2.

### 2026-10-23: Stage 393 (testing branch: KH2's drop-ahead row and the finisher tests' status bits)
- From the KH2 decomp (Steam): the "blocked line" is the mover's no-floor flag: row 1 on no floor ahead (`0x140402D20` /
  `0x1403BA750`, bit 0, `0x140172640`): `drop_ahead`, tested with KH1's sweep `0x1402C6E70` (Steam only) at attack_j_front +
  radius (assumption: KH2's length field). `+0x194` bit 5 (the finisher chain, assumption) and Berserk Charge during MP
  Charge (`+0x1DC` bit 11 = ability 107, `0x1403C1D10`) in `combo::condition`; a normal row under the combo length passes
  whatever the finisher count (the exe; the OG rewrite refused it). KH1 has no MP Charge, so Berserk Charge stays off.

### 2026-10-23: Stage 392 (testing branch: KH2's hop rows on KH1's air state)
- The tester: KH2's rows that hop to a target above. KH2's plain hop rows (0: 182, 1: 181): KH1's state 2 (`0x1402B1960`),
  count 1, its air swing row with KH2's motion, then KH2's height and speed on KH1's air fields
  (`Character::would_take_launch`, `Launch`, `shared::physics::kh2_hop`; the hop numbers from the OG rewrite's rule, not the
  KH2 exe). Needs the Aerial Combo style on KH2 too, else KH1's row is kept and logged.

### 2026-10-23: Stage 391 (testing branch: KH2's row selection on KH1's air combo)
- The tester: the same picker on the air combo. KH2's pick plays on KH1's air requests 0xCC / 0xCD / 0xCE (slots for 181, 182;
  183, 184, `kh2slots::swing_rows` with `air`); KH1's air rows 18 / 17 / 16 / 15 (`0x1402A6C70`) give way as on the
  ground (`Kh1Combo`), each row's request read from KH1's row table. KH1's air lunge and hop kept (asked).

### 2026-10-23: Stage 390 (testing branch: KH1's own ground rows give way to KH2's pick)
- The tester Mr Bugs (`docs/tester-requests.md`, "KH2's combo row selection", option (a)): at Base Combo KH2, where KH1's
  ground picker (`0x1402A6EE0`, Steam) took a row of its own (dash 0 / 1, row 2, 9 / 10, ability finishers 3..7, or the
  wrong kind) and KH2's pick is a plain ground swing or finisher, `combat::exec` runs KH1's plain row 13 / 11 or 8 instead
  (`combomods::kh2_row`); after a dash row the direction's y is put back to 0. His t374 trace f27098: KH1's dash (0xD0)
  where KH2 took row 5. Sora's full PTYA table (staged `00battle.bin`), KH1's row runner (`0x1402A68F0`) and row 2 (a
  drop ahead, `0x1402C6E70`; t374's notes called it a jump at a target above) in `docs/kh1-anim-slots.md` §7.

### 2026-10-23: Stage 389 (testing branch: KH2's walk and run baked)
- The tester: bake KH2's walk and run to compare. `kh2slots::CYCLES`: requests 0x1 / 0x2 are kind 3, three slots each on
  KH1's 2-4 / 5-7 (KH2's field cycle on the field slot, its battle cycle on the other two), redirected at the first;
  `Play::Slot` carries the field flag. Under Misc Animations. KH1's motion rate `+0x284` is set to 1 by `0x14029EF50`;
  otherwise only by event-script commands (decomp). Notes: `docs/kh1-anim-slots.md` §8.

### 2026-10-23: Stage 388 (testing branch: no hair switches)
- The tester: KH2 has no physics, its secondary parts are keyed, so the switches go. `hair_physics`, `kh2_hair` and
  `jiggle.rs` removed; KH2's animation of the hair, hood, sleeves, necklace and chain always runs on KH2's poses (the OG
  rewrite has no cloth or hair physics; no constraint in KH2's motions drives them). The keychain on a Dream weapon or the
  wooden sword keeps KH1's rest pose.

### 2026-10-23: Stage 387 (testing branch: KH1's own parts take KH1's own animation of the move)
- The tester: a baked motion for KH1-only parts, with KH1's animations as the reference. `retarget::KH1_OWN` (41 bones:
  belt chain, belt-front pieces, shorts, jacket hem, the necklace's pendant), `Retarget::set_own` / `own_relation`, the
  belt chain carried with its own links: they take KH1's own motion of the move KH2's stands in for, stretched to KH2's
  length. Live from KH1's pass (`pose::own_from_kh1`); baked from the template (`Baker::poses`, its slot from
  `kh2slots::bake_one`). Test `local_own`. Not done: matching the stride phase within a cycle.

### 2026-10-23: Stage 386 (testing branch: KH2's squash and stretch)
- The tester: carry KH2's scale keys. `Secondary::scales`, `secondary::apply_scale` (live, last step before the write),
  bake scale channels kinds 1-3. Scale is multiplied down the bones and stretches only each bone (KH1 `0x1403900B0` /
  `0x1401DB310` / `0x140394CF0`, decomp; KH2's side from the OG rewrite's evaluator). Today it carries only the sleeve
  flaps' scale (no other matched KH2 bone keys scale).

### 2026-10-23: Stage 385 (testing branch: KH1's necklace follows KH2's; the KH2 -> KH1 part list)
- The tester Mr Bugs (`docs/tester-requests.md`, "Missing sleeves animation"): the hem and shorts from KH2, only parts KH1
  has, and a list of every part shared between the models. From the files: KH2's model puts each vertex on one bone;
  the hood (72 / 73), sleeve flaps (113 / 115 / 205 / 207), spikes 143 / 159 / 161, the chest ring 129-135 / 221-227 / 70
  (KH1's hem layout), the trousers and the pelvis pieces carry no vertices, so the hem and shorts aren't copied from them.
- `Parts::necklace`: KH1 86 <- KH2 165 (which carries its mesh), measured against the chest, held off the chest. Full
  KH2 -> KH1 list in `docs/kh1-secondary-motion.md`.

### 2026-10-23: Stage 384 (testing branch: the form's run speed in KH2's camera rates)
- `camera.rs` passed a fixed run speed of 8; KH2 reads the player's `+0x12C`, so faster forms swing and follow
  faster. It passes the current form's KH2 run speed (or Sora's). Assumption: `+0x12C` is that run speed (the crate's
  notes). Notes: `docs/kh1-kh2-camera.md`.

### 2026-10-23: Stage 383 (testing branch: KH2's camera reset puts the eye on its goal; a cut while moving)
- `0x140165110`: a reset also puts the eye on its goal (only the turn eases); with the left stick held the cut
  (`0x140164520` with its cut flag, then `0x140164d70`) places the look point and eye with no spring. Which fields are
  the rendered points is inferred from that copy. `kh2-camera`. Notes: `docs/camera-model.md`.

### 2026-10-23: Stage 382 (testing branch: KH2's camera turns and zooms by the right stick's whole tilt)
- From the decomp (Steam KH2): the turn tests each axis's direction bit (`0x140178ab0` / `0x140178b10`, zoom
  `0x140178b50` / `0x140178b80`) and moves by the stick's whole tilt (`+0x2c`), so a full diagonal turns 3 deg and zooms
  8 a frame together; the follow camera zooms after a reset, the lock-on camera (`0x140176c70`) turns and zooms first.
  The swaps are option bits (`0x1402e5330`, `0x1402e5350`). Assumption: the direction masks (exe data) are the right
  stick's x / y bits. Notes: `docs/camera-model.md`.

### 2026-10-23: Stage 381 (testing branch: KH2 movement reads the stick KH2's way)
- `takeover::move_tilt` had its own round 20% dead zone (labelled KH2's, it wasn't); it and `stick_dir`'s fallback use
  `character_core::pad::stick` now, so gentle tilts walk Sora as in KH2. Open: KH2's walk / run split (`run_threshold`
  50% is a hypothesis) and the controller's own 0.1 `DEAD_ZONE` on top of the pad's.

### 2026-10-23: Stage 380 (testing branch: KH2's camera reads the sticks KH2's way)
- The tester: KH2's stick dead zone 1:1. `character_core::pad` (`stick`, `stick_vec`) follows Steam KH2 `0x14039c180`
  (60 of 127.5 off each axis, over 67.5, a direction and a tilt clamped to 1); `camera.rs` reads the pad's sticks
  that way and `STICK_DEAD` (15%, camera only) is gone. KH1's own processed sticks are built the same way
  (`0x14028b4f0`, dead zone 60 from `0x14028afc0`) and stay the stand-in for the keyboard and mouse. Notes:
  `docs/kh2-pad.md`.

### 2026-10-23: Stage 379 (testing branch: KH2's camera swing-behind test on the stick's direction)
- t373 correction (the tester asked to confirm in the KH2 decomp). From the decomp (Steam): `0x14039c180` keeps the left stick
  as a direction plus a tilt (`0x140142080` normalizes it), so `0x140165110` skips the swing within 25.8 deg of straight
  back at any tilt. `kh2-camera` tests and scales by that direction times the tilt (tests). Notes: `docs/camera-model.md`,
  `docs/kh1-kh2-camera.md` 9a.

### 2026-10-23: Stage 378 (testing branch: Frame Advance draws the held frames)
- The tester's t377 test: the game state held, but the picture was black on held frames (only the frames that ran
  showed). t372 held every task not flagged for a pause, and most of KH1's draws aren't flagged (from the decomp's
  registrations: flagged are 140000 and the model pass 160000, not the map, effects or HUD); KH1's pause shows the
  last frame instead.
- `frameadv::hold_tasks`: only tasks before KH1's first draw pass (priority under 140000: actors 80000, camera 110000,
  scripts and the other updates) are held; every task from 140000 on runs. The held tasks are logged once. Notes:
  `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 377 (testing branch: Frame Advance's period key hidden from KH1)
- The tester's rule (period key works the same way as R2): KH1 no longer sees the period key while Frame Advance is
  on. `frameadv::install` hooks KH1's keyboard mapping (Steam `0x1400D7E80 (input object)`, prologue
  `40 56 41 56 48 83 EC 48` read in the Steam exe), which clears DirectInput's period byte (0x34) in KH1's two key
  buffers (input `+0x12A0` / `+0x13A0`, filled by `0x1400DA9F0`) before KH1 maps keys to its pad. Steam only, like
  the rest of Frame Advance. Notes: `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 376 (testing branch: KH2 effects through KH1's own effect system, step 2: the element layer)
- The tester Mr Bugs (`docs/tester-requests.md`, "KH2 effects in KH1's own effect system"): KH2's element features in
  KH1's own effect system, not a layer beside it. Notes: `docs/kh2-effects.md` §16.
- From the decomp (Steam): KH1's effect instance has a start wait (`+0xD0`, game frames; `0x1401ECB50` /
  `0x1401ECF30`), a colour its draws multiply by (`+0x30..+0x3C`), an ended byte (`+0x91`) and a scale (`+0x20`);
  the root is the instance's times the particle header's local transform (`0x14031CEA0`). `0x1401EC470 (owner)` stops
  the instance whose `+0xB0` is that element.
- `kh2dpx`: Explosion's elements 46 / 47 get the KH2 element's wait (+0.1 bound) and scale on KH1's instance; their
  offset and half turn are folded into their own copy of effect 33's particle data (`kh1dpx::build_with`, numbers
  `0x700 | element`); KH2's motion changing (0x20) or Sora's object going (0x200) ends them the KH2 way: 0x40 fades
  the instance alpha over the fade-out frames, then KH1's stop (`0x1401EC470`, Steam), 0x80 sets the ended byte
  (`kh2dpx::tick` / `owner_gone` from `kh2fx`). `kh2_fx::math::xyz_i_of` (inverse of `euler_xyz_i`).
- Found, not yet used: KH1's object-root code 0x8000 and its point along the Keyblade (bone 0x30, t, binding mode 3
  `0x1402ADD60`) for KH2's 0x4000 / 0x4003 (elements 48 / 44 wait for steps 3-4).
- Kept KH1's: a bound element starts at wait <= -2 frames (KH2 has no such rule; asked).

### 2026-10-23: Stage 375 (testing branch: KH2's sleeve animation on KH1 Sora)
- The tester Mr Bugs (`docs/tester-requests.md`, "Missing sleeves animation"): under KH2's moves Sora's sleeves didn't
  move. His t371 runs (KH2's run, then KH1's) log no bones; the cause is in the files: KH1's sleeve chains (128-143,
  232-247, four per arm from the upper arm 93 / 198) were in no retarget part and not in `secondary`, so they kept
  their bind relation to the arm. Both games key their sleeves by hand (KH1: 59-93 motions; KH2: root 110 / 202 and
  flaps 113, 115, 205, 207, no constraints in any motion).
- `secondary.rs`: `Parts::sleeves` (in `ALL`, `covers` 128-143 / 232-247). Each chain's shoulder joint takes KH2's
  sleeve root's (111 / 203) model-space turn, carried by the swing from KH2's upper arm onto KH1's; the joint down the
  sleeve takes the flap's turn relative to that root. Four KH1 chains blend KH2's two flaps by place round the arm
  (front / back 98%, under / top about half). Live (`pose`) and in the bake (`Baker`), with the hair's switches.
- Checked on the files, all 89 KH2 motions: no NaN, lengths kept, links within 0.03 deg of KH2's, sleeves within 29 deg
  of the arm (KH1's own motions: 41 deg). Not carried: KH2's flap scale keys. Notes and the other parts still rigid
  (shorts panels, hem, belt chain, necklace): `docs/kh1-secondary-motion.md`.

### 2026-10-23: Stage 374 (testing branch: KH2's row selection on KH1's ground combo)
- The tester Mr. Bugs (`docs/tester-requests.md`, "KH2's combo row selection"): KH2's row selection, so the right swing
  plays for the target's distance; the swing order was fixed (0xC8 / 0xCA 151, 0xC9 152, 0xCB 154).
- From the decomps (Steam): KH2's selector `0x140402D20` (row checks `0x140402750`, finisher / normal tests
  `0x1403D7780` / `0x1403D7850`, range `0x1404029F0` -> `0x1404027F0`, walk with anti-repeat `0x140402C50`, row 9 / 5
  fallback) is `character_core::combo::select`, except the range gap, which the exe never lets go below 0
  (`combo::in_range` now clamps it). KH1's Attack step `0x1402A7510` and ground picker `0x1402A6EE0` mark and walk its own
  rows the same way; its plain swings are rows 13 / 11 (0xC8 / 0xC9) and its finisher row 8 (0xCB).
- `Character::would_take_after`: the host's playing motion goes into the anti-repeat (`kh2_would_take` takes it;
  stage 312's "always row 5" log). `combomods::select_swing`: at Base Combo KH2, a pick that fits KH1's plain swing or
  finisher (ground, no hop, no ability) points that request at our slot of the picked motion (`kh2slots::pick`, before
  KH1's row requests it); `kh2slots::swing_rows` builds a slot for each such motion per request. KH1's row, hits and
  damage stay KH1's. `shared::swing_travel` takes the playing slot's motion. Notes: `docs/kh1-anim-slots.md` §7.
- Not changed: KH1's dash / jump-at-target / ability-finisher rows, KH2's hopping rows, the air combo.

### 2026-10-23: Stage 373 (testing branch: KH2's camera no longer circles Sora when he runs toward it)
- The tester Mr. Bugs (`docs/tester-requests.md`, "KH2 Camera fixes"): running backwards turned the camera and Sora in
  a circle. His trace f571370: left stick straight down, KH2's camera yaw and Sora's yaw both turned 0.0175 a frame
  (frames 571250-571262, 571287-571370): the swing-behind formula, 1 degree a frame.
- Cause: `camera.rs` passed KH1's stick (y positive pulled back) to `kh2-camera`, whose y is forward, so its "not pulled
  straight back" test read straight back as forward. `camera.rs` now negates y.
- `kh2-camera`: the swing test and rate use the stick's own components, not its unit direction. Checked in the exe
  (Steam KH2 follow update `0x140165110`, pad sticks `0x14039c180`): no swing while the pad's y is 0.9 or more pulled
  back; diagonally back or a lighter tilt still swings, as in KH2. Tests for both.
- Not changed: KH2's per-axis stick dead zone (60 of 127.5, `0x14039b7f0`) against ours (round 15%). Notes:
  `docs/camera-model.md`, `docs/kh1-kh2-camera.md` 9a.

### 2026-10-23: Stage 372 (testing branch: Frame Advance holds the actors)
- The tester's t371 test (`testers/frame-advance-in-the-debug-menu`, 8 traces at 30 / 60 fps, KH1 and KH2 moves):
  only effects held and stepped; Sora's physics frames went on every frame while the mod's clock stood still. The
  pause machine (`0x14017F150`, every frame before the task list) sets the task mask each frame itself, so t371's
  write was overwritten; `0x1401ED6D0` pauses the effect instances (pool `0x142678280`), not models.
- `frameadv`: while the game is held, every task of KH1's main list (`0x142867380`, 0x18-byte tasks: `+0` in use,
  `+2` flags, `+8` next, `+0x10` function) in use and not flagged 1 (the flag KH1's pause runs) has its function
  swapped for `frameadv::skip` (does nothing, keeps the task), and gets it back for a frame that runs; the list is
  walked every held frame, so new tasks are held too. The actors are one of those tasks (`0x140298940`, priority
  80000, runs the actor list `0x142D5AB00`). The mask is no longer written. Data writes only, no hook. Notes:
  `docs/kh1-frame-advance.md`.

### 2026-10-23: Stage 371 (testing branch: Frame Advance in the debug menu)
- The tester Mr. Bugs (`docs/tester-requests.md`, "Frame advance in the debug menu"): a debug menu row that holds the
  game; R2 or the period key plays one frame, held 1 s plays at full speed until let go; works at any fps; R2 does
  nothing else while it's on; a button held over several steps is one press, one let go is released on the next step;
  the trace's seconds count game frames, with a Game Frames row on the Trace Recorder page (option b: off by default,
  turned on with Frame Advance). Notes and addresses: `docs/kh1-frame-advance.md`.
- `frameadv.rs` (new): from KH1's frame end (`debugmenu::frame_end`, after the frame's task list) the next frame is
  held or run the way KH1's pause holds it: the task mask (Steam `0x142867370`: 1 = only the pause-flagged tasks,
  0 = all) and the model objects' pause (`0x1401ED6D0`). KH1's pacing runs every frame, so a stepped frame has the
  frame scale of the fps setting. Before a stepped frame KH1's pad record's held buttons and repeat timers are put
  back to the last frame that ran. Left to KH1 while its pause machine is up. Steam only (the row says so elsewhere).
- R2 is taken off every pad record while it's on (`probe::pad`, and `takeover::RAW_BUTTONS`); the period key
  (`GetAsyncKeyState`, game window in front). Not done: hiding the period key from KH1's keyboard mapping.
- The mod's clock (`takeover::now_ms` -> `frameadv::clock_ms`) stops while the game is held; the draw's "posed
  lately" checks (`model`, `pose`, `leftblade`) take `frameadv::age`; `takeover::paused` is false while held.
- `trace`: Game Frames (`trace_game_frames`, `trace::game_frames`): the clip's window is `seconds` x 60 ticks
  (`Row::ticks`), else real time (`Row::wall_ms`, new). Select + Triangle writes the trace while held
  (`trace::frozen_combo`). Debug menu: main page Frame Advance row, Trace Recorder page Game Frames row.

### 2026-10-22: Stage 370 (testing branch: KH2's HD textures matched by the right header word)
- The user's t369 run: Explosion still has the squares. The log: the DPX was entered in KH1's loaded files, but with 0
  HD textures, every texture "none". The asset headers' `+0x20` words (P_EX100: `0x850A0`, `0x87120`, ...; texcommon:
  `0x424D0`, `0x62550`, ...) each step by the asset before's size (8320, 131200 bytes): the asset's own place in the
  stream, not the replaced data's offset (stage 369's assumption, wrong).
- `HdAsset` keeps both words: `pkg_offset` (`+0x20`) and `offset` (`+0x24`), and textures are matched by `+0x24`.
  Checked in the exe: KH1's own loader reads the same header's `+0x24` word as an HD entry's key (`0x1400D53F0`, low
  24 bits kept, `0x1400E1680`). The log line now lists each asset as "name for <offset> (size, at <pkg_offset>)".

### 2026-10-22: Stage 369 (testing branch: KH2 effects through KH1's own effect system: their HD textures)
- The user's t368 test: Explosion's wrist flashes showed as squares at Sora's body, the rest fine. The log: the DPX was
  built and relocated and both elements started, so placement and programs ran and only the textures failed.
- From the decomp (Steam): KH1's PC port never draws a DPD's PS2 texture data. Its DPD texture pass (`0x14031EC10`,
  one texture per TBP) asks the HD texture lookup (`0x14010C050`) for each record's pixels (`+0x20`): the loaded file
  holding the pointer (`0x1400E2B20`: KH1's loaded-file list, each entry's data `+0x658`, size `+0x640`), then that
  file's HD entry (`+0x670` list) whose `+0x54` is the pixels' offset in the file (the offset keying is on: set at
  start in `axaAppMain`). Our buffer was in no file, so there was no texture. KH2's PC port does the same with its
  own HD images (DDS, DXT5, as KH1's), stored with each file.
- The user chose option A (native): the DPX is entered in KH1's loaded-file list the way KH1 enters a file it loads
  (`0x1400E1680` / `0x1400E2F60`: a 0x688-byte resource entry, its data, size, name, MD5 (KH1 keeps a file's textures
  while a file with that MD5 is loaded), and 0x78-byte HD entries), each texture record given the KH2 HD image KH2
  draws for it. KH1's own loader makes the textures. Vtables are copied from KH1's own entries, memory comes from the
  game's CRT (`malloc`, `_aligned_malloc`), the list from the lookup's own first instruction (no new address).
- `kh2_formats::egs::Archive::read_remastered_all` / `kh2_data::Install::read_remastered_any`: a file's HD assets with
  each header's `+0x20` word. **Assumption** (OpenKH's `RemasteredEntry.Offset`): the offset in the original file of
  the data the asset replaces; the log lists each asset and which texture it matched, or "none".
- `kh1dpx::build`: also rewrites each texture record's own TEX0 (`+0x10`, which KH1's records keep equal to `+0`/`+2`),
  and returns each texture's origin (own or common, its record's offset there, its TBP, its offset in the DPX), the
  source DPDs' places and the DPX's MD5; `texture_records(dpd)`. A texture takes the HD image of its source TBP (the
  asset at any record with that TBP: one image per TBP in both games).
- **Assumption**: KH1 adds to its loaded-file list on the game thread (ours is added there, on the first native start).

### 2026-10-22: Stage 368 (testing branch: KH2 effects through KH1's own effect system, step 1)
- The user: port KH2's effects into KH1's systems so any move can use them; ladder step 1 (native), bones by option A
  (as the base attacks), Explosion first. Plan and checks: project notes `kh2-effects-in-kh1.md`.
- Checked (Steam decomps and the files): KH1 and KH2 share the effect engine (DPX `0x82`, DPD `0x96`, programs 1..235
  the same by name and number; KH2 adds 236..242) and the 30 Hz effect clock. What differs is texture memory: KH1's
  own-texture window starts at TBP `0x3800` / CBP `0x3D40` (10 / 6 slots), KH2's at `0x3000` / `0x3E00` with its common
  textures below.
- `kh2_formats::kh1dpx::build(pax, texcommon, effects)`: a KH1 DPX of some effects of one KH2 DPD. Keeps their particle
  datas only; the textures they draw with (the DPD's own and KH2 common ones) packed from KH1's window (textures sharing
  a TBP get the largest one's space); shapes' and models' TEX0 moved; texture masks rebuilt; programs above 235 set to
  KH1's dummy program until plugged in. Explosion (effects 31..34): 4 particle datas, 12 textures, 6 TBP / 2 CBP slots.
- `kh2dpx` (new): builds Explosion's DPX in the background from the KH2 install; on its first use KH1 relocates it
  (`0x1401E0080`), and elements 46 / 47 (effect 33, KH2 bones 170 / 78) start with KH1's start-on-bone (`0x1402ADCF0`)
  on Sora's KH1 bones 93 / 198 (`0x2000 | bone`). KH1 steps, binds, textures, draws and ends them; nothing of ours runs
  per frame. Kept inside one 32 MB handle region. `kh2fx::event` hands those two elements over first; while the DPX
  isn't built, or KH1 has no free element, `kh2fx` plays them as before.
- Ini `kh2_fx_native` (on): off = `kh2fx` plays everything as before.
- Not yet (step 2): the elements' delay, fade-out, end-with-motion and their half turn about z (KH1's bone binding
  doesn't take an element's rotation). Explosion's elements 44, 45 and 48 still play through `kh2fx` (they need KH2's
  `pppRyjMegaBirth` and `pppEiDrawShipoly`, steps 3 and 4).

### 2026-10-22: Stage 367 (testing branch: the baked swing travel at KH2's full distances)
- The user: the baked travel at KH2's full distances, to match KH2's travel (t365 scaled it by the retarget's hip
  ratio, a choice made without asking). Keeping the swing-travel feed off for baked swings is fine (the user).
- `Baker::poses`: the travel is KH2's root travel along the ground as it is, no ratio. Sora's placement has scale 1
  (t364's KH1 trace: swing 98's H10 steps 1.3 and 6.32 a frame, moved 1.30 and 6.32), so H10 moves him 1:1. The body
  carries the same, so the model stays on him; his feet may slide a little on KH1's shorter legs.

### 2026-10-22: Stage 366 (testing branch: t365 builds)
- t365's build: E0592, `Retarget::hip_ratio` defined twice; `Retarget` already had it. The added copy is removed.

### 2026-10-22: Stage 365 (testing branch: KH2's swing travel baked into the motion, KH1 moves him by it)
- The user's t364 traces: the finisher no longer turns, but in KH2's baked swings the model trails the screen's centre
  and jumps back to it at the swing's end, while KH1's own swings look right. Swing totals: KH2's 230 / 182 / 221 / 330
  units, KH1's 92 / 89 / 95 / 186.
- From the decomp (Steam), correcting stage 363: KH1 does take travel from the motion. A request entry's bit 15
  (Sora's ground combo, 0xC8..0xCB; not his air combo) becomes `0x4000000` (`0x1402A01E0`) and the actor's `+0x160`
  bit 4 (`0x14029FC40`). The pose pass (`0x140391C70`) writes H10's translation to the instance's `+0x100` and the
  world position plus it to `+0xF0`; the actor copies `+0xF0` to `+0x50` (`0x1403948D0`) and keeps this frame's step
  in `+0x200` and the travel so far in `+0x210` (end of `0x14029F0A0`). The move function (`0x14029A3C0`, which calls
  the ground step `0x1402B1850`) adds `+0x200` to the displacement; the draw pulls the placement back by `+0x210`, put
  back and cleared at the motion's end (`0x14029FC40`). KH1's own swings carry the same travel in H2, which the body
  hangs from (98..101: H10 z 100 / 120 / 100 / 190), so the model stays on him.
- Ours kept the template's H10 (KH1's travel) with the travel taken out of the body, and `ground_step` fed KH2's
  travel too: he moved by both, and the model was pulled back by KH1's travel with nothing to cancel it.
- `kh1_data::bake::baked` takes `travel` (per frame, model space): the template's H10 translation channels are
  replaced by it (linear keys, reduced). `Baker::poses(.., travel)` puts KH2's root travel along the ground (from
  frame 0, times the retarget's hip ratio, `Retarget::hip_ratio`) back on the whole pose and returns it.
  `kh2slots::prepare` bakes it for the ground combo when the request has bit 15 (`travels`); `Move.travel`,
  `moves_itself(actor)`.
- `shared::physics::ground_step`: for a move that moves itself, KH2's swing travel isn't fed (KH1's step and its own
  root travel move him). The t363 / t364 feed stays for unbaked swings. Air combo unchanged.
- Travel scales with the hip ratio (no foot slide on KH1 Sora's shorter legs), so lunges are shorter than KH2's by
  that ratio and Sora's world scale.

### 2026-10-22: Stage 364 (testing branch: the finisher no longer turns toward KH2's travel)
- The user's t363 traces: KH2's finisher (154) turned Sora mid-swing (0.196 rad a frame, twice) and ran him off the
  screen's centre, then snapped back; swings 151 / 152 unchanged; the stick plays no part (the user).
- From the decomp: KH1's player control (`0x1402A3300`) runs the state's input filter (`0x1402A7B30` -> state 0's
  `0x1402A5F10`): with the stick let go it takes the move vector `+0x20` as the input; the ground pre-step
  (`0x1402CC1A0`) then sets the desired facing `+0x150` from it (`0x140297FB0`, atan2) and KH1 turns his facing
  toward that at the stats' rate (`+0xD0`, π/16 in an attack). t363 left KH2's travel direction in `+0x20`, sideways
  in the finisher, so he turned toward it.
- `ground_step`: KH1's move vector and speed (`+0x20..+0x2C`) are saved before KH2's travel is written and put back
  after KH1's step, so KH1's step still moves him by KH2's travel and nothing else sees it.
- Not explained yet: the ~26 units a frame while he turned and the 28-unit snap back (a new trace will show whether
  they went with the turn).

### 2026-10-22: Stage 363 (testing branch: KH2's swing travel through KH1's own move fields)
- The user: KH2's ground combo travel (physics KH2) into KH1's systems, the goal being every KH2 system inside KH1's
  while keeping KH2's feel. From the decomp: KH1's ground state step (`0x1402B1850`, Steam) moves him by his move
  vector (`+0x20`, laid on the ground normal `+0xC0`) times his speed (`+0x2C`) times the frame; the motion's travel
  helper (H10, the instance's `+0x100`) isn't read by it or by the combo, so baking travel into the motion wouldn't
  move him.
- `shared::physics::ground_step`: in KH1's ground swing (slots 98..101, action flag 4) at physics KH2, KH2's travel
  for that swing (the same clip root, stretch and facing as before) is written as KH1's move vector (flat) and speed
  (units a 60 Hz frame) before KH1's step runs, and KH1's step moves him (on slopes along the ground, as KH1 does).
  It used to overwrite the step's result afterwards.

### 2026-10-22: Stage 362 (testing branch: the baked swings hold the Keyblade teeth up)
- The user's t353 test: the baked combo slots play (log "KH1 plays ours", trace `draw_route` baked) and everything
  translates except the Keyblade, held the wrong way round (teeth down), as expected from the raw bake.
- `Baker::poses` turns the grip and everything under it half round about the blade (`turn_grip`, split out of
  `keyblade_teeth_up`) before the keychain, as the drawn path does, while `keyblade_turn` is on. Every weapon gets it
  in the bake, the dream weapons and the wooden sword too (the user: they show the same issue; option A).

### 2026-10-22: Stage 361 (testing branch: the debug menu's equip crash)
- The user: the game crashes equipping another weapon from the debug menu. The log: `CRASH 0xc0000005 at exe+286737`,
  inside KH1's weapon change (Steam `0x140286720`) at its first use of the id (the item record lookup
  `0x14028F970`, which takes all of `ecx`); `rcx` was `0x2c296451`: the id `0x51` in the low byte, the rest left over
  by the byte argument the t360 call declared. The id is passed as a whole `u32` now.

### 2026-10-22: Stage 360 (testing branch: the debug menu equips Sora's weapon)
- The user: stuck on the Dream Sword after the debug menu gave it ("game won't let me unequip dream weapons"; the log
  shows weapon `0x52` from the Equipment page on): KH1's Equipment page refuses Sora's weapon row while he holds
  `0x52..0x55` (`0x1402E6C60` / Steam `0x1402E90A0`, a four-id list read from the exe). Then: "if its possible to let
  sora equip weapons via the debug menu lets do that".
- **Weapons page**: Cross now equips the weapon on the row (one is given first when Sora owns none) through KH1's own
  weapon change `0x1402845A0` (Steam `0x140286720`; `steam_map.in` gets the pair), which writes his weapon byte and
  loads and makes the weapon object, then the Status copy's refresh `0x1402E5A80`. Left / right still change the count.
  The equipped one is marked. The live stats block follows at KH1's next rebuild (the pause menu's exit, an area).

### 2026-10-22: Stage 359 (testing branch: the debug menu's Weapons and Abilities pages)
- The user: under the Stats page, a weapon and an ability page "so I can add or remove each weapon and ability".
- **Weapons**: every Sora weapon (item kind 2 in KH1's item table) with its inventory count (`save + 0x499 + id`,
  `docs/kh1-equipment.md` section 2; equipping changes no count): left / right add or take one (0..99, never below
  the copies equipped across the members), Cross gives one or takes every free copy. The equipped one is marked.
- **Abilities**: Sora's KH1 abilities (1, 2, 5..28, 53..65: not the party's 29..52, not Glide / Superglide) and ours
  (`growth::MENU`, not KH2's Glide: the Glide style swaps those), each with its copies in his lists and how many are
  on (`save + 0x44`, 48 bytes, bit 7 = off; High Jump and Mermaid Kick in the shared `+0x599`, 10): right adds a copy
  (off, as KH1 learns), left takes one (an off copy first), Cross adds one or takes them all; a full list refuses.
- What the page adds or takes out is left alone for the session by the mod's upkeep (`menu::Growth::update`): KH2's
  abilities given by level / boss / namesake aren't given back once taken out, an added Aerial Dodge isn't held back
  until Genie Jafar, an added Zantetsuken stays, and at `abilities = test / all` added ones of ours aren't stripped
  (`debugmenu::ability_forced`).
- Long pages scroll (16 rows around the cursor, the title shows the place).

### 2026-10-22: Stage 358 (testing branch: the debug menu's stats reach the save too; Level)
- The user: the Status page's Level is `exe+2DE9364` (Steam), which the decomp shows is Sora's save record's `+4`
  (the save block `0x142DE9360` Steam, `0x16C00` bytes, written to and read from the save file), read by the
  Status page itself (Steam `0x140312410`, `0x14031272D movzx eax, [r13+4]`, r13 = save + member x 0x74; the
  page's EXP is the record's `+0x40`, its HP / MP / Strength / Defense / AP the Status copy's). Then: "touch saves
  as well, and any equivalent values let our debug menu adjust with the same settings".
- `debugmenu`'s Stats page has Level (1..100, save only) first, and every stat is written to Sora's save record as
  well as the world block and the Status copy, by edits and by locks (every frame). The record holds HP (`+5`) and
  MP (`+7`) as they are, and base max HP (`+6`), max MP (`+8`), AP (`+9`), Strength (`+0xA`), Defense (`+0xB`)
  before equipment (KH1's stats function adds the weapon's and accessories' and caps Strength, Defense, AP at 100):
  for those the record gets the value less the equipment's bonus, read from KH1's own stats function (unhooked,
  `menu::kh1_stats`) less the record's byte, so KH1's next rebuild gives the value back. Bytes, 0..255.

### 2026-10-22: Stage 357 (testing branch: the debug menu follows KH1's confirm button)
- The user (Circle to confirm): the debug menu's confirm and back buttons follow the game's setting. KH1 keeps them
  as pad masks, confirm at `0x140546390` (Steam `0x1405456A0`, the Config row's `CONFIRM_MASK_RVA`) and cancel at
  `+4`, both set by `0x1402F6700` (Steam) from the setting: Circle `0x2000` / Cross `0x4000`. `debugmenu::buttons`
  reads them each frame (Cross / Circle if they hold anything else); the hints show X or O to match.

### 2026-10-22: Stage 356 (testing branch: the debug menu's crash as it opened)
- The user's t355 log: the game crashed the frame the menu opened, in KH1's command-list getter (Steam
  `0x1400FD238`, reading `0x84`: the renderer's command buffer `+0x5988` null). The menu was drawn from KH1's frame
  end (`0x140109B40` Steam), and by then the field loop has already called the end of the frame's drawing (Steam
  `0x1400FBBE0`, which closes the frame's list with `0x1400F7160`): nothing is recording.
- The menu is drawn from a hook on that function's start instead (Steam `0x1400FBBE0`, Epic `0x1400FBD80` from the
  table's neighbours, prologue `40 56 57 41 56 48 83 EC 40`; the field loop `0x140170D50` calls it after the HUD's
  tasks), once a frame (KH1's frame end calls it again). The frame end keeps the menu's input and the stat locks.
- `fx12::begin` gives up when the renderer has neither a cached list (`+0x5990`) nor a command buffer (`+0x5988`),
  so no draw of ours can reach the getter with nothing open.

### 2026-10-22: Stage 355 (testing branch: t354's build error)
- The user's build of t354: `takeover::trace_frame`'s `trace::Row` lacked the new `cats` field (E0063). It is set to 0
  there, as `trace::frame` fills it.

### 2026-10-22: Stage 354 (testing branch: the debug menu: Trace Recorder with categories, Stats with locks)
- The user: a debug menu opened with a button combination, drawn over the game like the command menu (the game
  runs on), a grey box with some opacity, white text, the selected row yellow; turned on in the Config menu; pages
  that replace the main one, Back out of them, Exit on the main one; Sora not moved by the buttons while it's open.
  `debugmenu.rs`.
- **Opening**: the Config menu's new Debug Menu row (Off / On, ini `debug_menu`, off by default). L1 + R1 + Triangle
  opens it and closes it; closed that way it opens again on the page it was left on; Exit (or Circle on the main
  page) closes it and the next open starts on the main page. While L1 + R1 are held with the menu on, KH1 doesn't
  see Triangle.
- **Input**: `probe::pad` hands the merged pad to `debugmenu::pad`; while the menu is open (and after it closes,
  until every button is let go) KH1's records get no buttons and centred sticks, and KH2's `RAW_BUTTONS` /
  `RAW_STICKS` read nothing. The menu reads the pad as pressed: D-pad up / down, left / right (repeat when held),
  Cross picks, Circle goes back. (Controllers only: the keyboard path isn't covered.)
- **Drawing**: a hook on KH1's frame end (`0x140109CE0 (renderer)`, Steam `0x140109B40`, prologue
  `48 8B C4 57 48 81 EC 90 00 00 00`, from the Steam decomp and exe: `0x1400DAFF0` -> `0x140109B20` -> it, before the
  resolve, full-screen copy and present) draws it over everything that frame (HUD included) with
  `fx12::draw_overlay`: screen-space quads (view identity, a pixels-to-clip projection), no depth test, alpha
  blended; text from a font atlas (DejaVu Sans Mono Bold, 14 x 28 cells, `debugfont.rs`, run-length coded) kept with
  the model textures. `fx12` also uploads effect textures handed over after the renderer came up (the overlay can
  bring it up first: `late_textures`).
- **Trace Recorder page**: Trace (Off / On / Always), Seconds (1..120, steps freely), Sound Log (On / Off), and
  Categories: six groups (KH1 Actor, KH2 Controller, Pose, Camera, Input & Config, Events) whose rows turn all of
  the group on or off with left / right and open with Cross to set each of the 13 items (`trace::ITEMS`,
  `GROUPS`). Every change is written to the ini (`trace`, `trace_seconds`, `sound_log`, new `trace_*` keys, all on
  by default) and is live at once (`trace::set_*`; `trace::refresh` takes the ini's again when it reloads).
- **Trace categories**: a category that is off isn't gathered (the camera's `record`, the trace's draw / feet
  reads, the feet hook, KH2's side and Form in `takeover::trace_frame`, the hit-effect lines' formatting) and its
  columns leave the CSV (the header is built from the categories; each row keeps the categories it was made
  with, so a dump across a change writes empty cells where a row lacks a column). Log lines, hit-effect lines
  and the sound log's lines are separate categories of the events column (`log_as`, `trace::note_as`).
- **Sound log**: its hooks go in whatever the ini says, and write only while it's on (`trace::sound_log`), so it
  switches in game.
- **Stats page**: Sora's HP, Max HP, MP, Max MP, Strength, Defense, AP (left / right change, Cross locks). From the
  Steam decomp of KH1's stats function (`0x1402A5130`, Epic `0x1402A2FA0`): the stats block's `+0x3C` HP (at most
  max HP), `+0x40` max HP, `+0x44` MP, `+0x48` max MP, `+0x4C` Strength, `+0x50` Defense, `+0xE4` AP, `+0xC8` the
  save record it came from; KH1 fills the Status menu's copies at `0x142E94BC0 + member * 0x100` (Steam
  `0x142E941E0`, `0x9E0` below as its neighbours in the table) and Sora's world block (actor `+0x6C`) from the save
  and copies his HP / MP back into the save after. Changes go to the world block and the Status menu copy, never
  the save; locked stats are written to both every frame, after KH1's stats function fills a block for Sora
  (`menu::stats` -> `debugmenu::lock_stats`) and after Valor's stat step (`valorstats::step`). HP and MP stay within
  their maximums; a locked HP above 0 clears the block's dying flag (bit 4 of `+0`, as KH1's HP change does). The
  locks are not kept in the ini.

### 2026-10-22: Stage 353 (testing branch: KH2's slots added where KH1 binds Sora's motion sets)
- The user's t352 log: the set on Sora's actor in Traverse Town is the world's (about 88 slots, the combo at
  61-67), not his own 118, and its request table has 0xFFF for what it lacks. From the decomp (the user: look
  first): his own set is bound at the actor's creation by `0x140287E40` (Epic `0x140285CC0`), a world's on
  entering it by `0x14029E850` -> `0x1402886F0` (Epic `0x140286570`, from the neighbours, unchecked: the prologue
  test keeps it unhooked if wrong), his own saved at `+0x1E0`. `docs/kh1-anim-slots.md` §2.
- Both binds hooked (`kh2slots::install`): after KH1 binds a set onto Sora's actor (its model 293 bones), the
  slot and request handles become ours for that set, made once per set: its own slots (counted as the words before
  its first motion: no end marker in memory), ours right after; its request table with the requests our moves stand
  in for pointed at ours by style, only where the set's own entry holds the KH1 motion ours stands in for (by
  signature). Each set logged ("Sora's set ...: N slots of its own, ours at ...").
- `frame` no longer swaps or checks against the file: it extends `+0x1D0` and `+0x1E0` if a bind came before our
  motions were built, sets the redirects by style in every set, and ticks. Our moves are found by their motion's
  address (`current`), so `kh1_time`, `tick` and the trace work whatever slot number the set gives them.
- Removed: `kh1_own`, `describe_slots`, the per-frame table copy and the 118-slot assumption.

### 2026-10-22: Stage 352 (testing branch: KH1's slot table taken as its 118 entries; how KH1 lays it out, logged)
- The user's t351 log: KH1's slot table on Sora's actor has empty entries 80-88, 91, 104-107, 116, and no end marker
  after entry 117 (entry 118 = 02000000, not a handle; the "135 entries" counted data past the table). The decomp
  (`0x140287E40`, `0x1401D6230`): the actor's `+0x1D4` is the bound file's MMTN `+0x30`, its list at MMTN `+0x40`.
- `kh1_own` no longer asks for an end marker: the table is taken as the file's 118 entries (ours keeps its own end).
  Once per table it logs (`describe_slots`) which entries are empty, which hold another file slot's motion (by
  `pose::file_slot_of_motion`, the signature `slot_of` uses), which hold something not in Sora's file, and the
  words after entry 117. The request table test stays.

### 2026-10-22: Stage 351 (testing branch: KH2's slots go on Sora with KH1's empty slot entries)
- The user's t350 log: Sora's actor refused because KH1's slot table (in memory) has entry 80 of 118 empty, though
  the file has a motion there (`kh1_own`'s first test). The request table wasn't reached.
- `kh1_own` no longer requires every one of KH1's 118 entries: empty ones are copied as they are (logged once, which
  ones); only an all-empty table is refused. The slot table length and request table tests stay, with their reasons.
- Ours keeps KH1's own entries up to date each frame from KH1's table on the same actor (`Placed::kh1_slots`), so an
  entry KH1 fills after ours was made isn't left empty in ours.

### 2026-10-22: Stage 350 (testing branch: why KH2's slots aren't put on Sora, logged)
- The user's t349 run: all seven KH2 motions baked (KH1's FK within 0.22-0.30 units of the poses), but no slot of
  ours ever played: `kh2slots::frame` found Sora's actor "showing another motion set" every frame (logged 5 times,
  never "on Sora's actor"), so KH1 played its own 98-104 with KH2's swings drawn over them (trace `draw_route`
  follow, `Swing`). Tier A never ran in game either.
- `kh1_own` now says which test fails: the tables unreadable (handles and addresses), an empty slot table entry,
  the slot table's length if not 118 (with entry 118), or the request table's entries that differ from the file
  (count, first 12 with file and memory values). Each new reason is logged ("not taken (...)"), 40 at most.

### 2026-10-22: Stage 349 (testing branch: KH2's slot motions baked into KH1 motions, tier B)
- The user: bake KH2's animations into KH1 Sora's motions (`docs/kh1-anim-slots.md` §1.6, §3.1), a raw bake first
  (none of our corrections: Keyblade teeth-up, `flip_weapons`, our feet, `held`), behind an ini switch, then test
  in game. The Keyblade is part of KH1 Sora's skeleton (bones 280-292, its mesh skinned onto them), so KH2's body
  and Keyblade motions bake into the one KH1 motion; Valor's second Keyblade later.
- `kh1_data::bake::baked(template, frames, events, skeleton, poses)`: KH1 model-space poses per frame -> a KH1
  motion on the retimed template. Joint words plain FK (word 0; bit 25 where nothing moves; KH1's skipped leaves
  keep theirs), each bone's local as Euler X->Y->Z (continuous across frames) and translation, bind values left
  out, constants for held values, linear keys reduced within 0.001 rad / 0.02 units; the template's helpers and
  helper channels kept (H10's travel moves him as in tier A); KH1's FK from the channels checked against the
  poses (logged as "within N units").
- `pose::Baker` (loader thread): KH1's skeleton from `xa_ex_0010.mdls`, the retarget with the battle / field idles
  as rest, `secondary`; `poses` draws each frame as a tier A slot does (KH2's body and Keyblade, root travel out,
  retargeted, KH2's hair, hood and keychain).
- `kh2slots::prepare(mset, kh2, bake)`: with `kh1_bake = on` (new ini key, default on) each slot's motion is the
  baked one (`Move::baked`), else the retimed template as before; a failed bake falls back to the template.
- `pose::follow`: a baked slot draws nothing (KH1's own pass shows it); KH2's pose at KH1's time is still noted
  for KH2's effects on his bones (`kh2fx`, `reflect`). Trace `draw_route` "baked" with its motion and frame.
- Legs are FK in this first bake: no KH1 foot lift on slopes during these moves (tier A had none there either).
- Nothing tested in game; nothing compiled here (no crates in this environment).

### 2026-10-22: Stage 348 (testing branch: t347's trace row builds again)
- t347 didn't build: `takeover`'s `trace::Row` initializer lacked the new `feet` field (filled by `trace::frame`
  from `pose::take_feet`, as `draw` and `cam` are). Given its default there.

### 2026-10-22: Stage 347 (testing branch: the trace shows what KH1's own skeleton pass does to his feet)
- The user, before the tier B bake (`docs/kh1-anim-slots.md` §3.1): test in game whether KH1 bends Sora's legs
  onto a slope, since a plain-FK baked slot turns KH1's two-bone leg IK, and with it its ground lift, off.
  From the decomp (§1.6): `instance + 0x14` bits 0-1 turn the ground lift on; bit `0x200000` is set when a
  foot target was lifted onto the floor.
- `pose::note_feet`, straight after KH1's pass (`0x140391E20` or `0x140392710`) and before anything of ours,
  for Sora's own instance while the trace runs: that word and the ankles' heights (bones 19 and 45).
- Trace columns after `kh2cam_lock_dist`: `ik_ground_on` (bits 0-1), `ik_lifted`, `ik_flags`, `foot_r_up`
  and `foot_l_up` (each ankle above `floor_up`, the floor under his centre), `pose_path` (which pass ran since
  the last row, with counts; empty columns when KH1 didn't pose him).
- The test: Insert to "off (KH1's own animation)", stand side-on on a slope or stairs, Select + Triangle.

### 2026-10-22: Stage 346 (testing branch: KH2's pose and effects once per game frame, the game's own frame delta)
- The user: KH2's side should run on the game's own delta (30 fps base, 60 fps and uncapped on PC), and the
  hook is heavy on low-spec hardware (at 30 fps every KH2 step ran twice, at a hitch up to four times).
  From the Ghidra decomp of the Steam exe (`FUN_140154740`, `FUN_140154700`, the pacing thread at ~line
  255080; project note `kh2-frame-delta.md`): the delta (`0x140717480`) is always a whole number of 60 Hz
  units, one update of delta 2.0 at the 30 fps base, no fractional delta at higher or uncapped rates (those
  only add presents between logic callbacks, ~59.94 Hz at most).
- Kept: the controller, KH1's mover, hits, guard windows and sounds still step in whole 60 Hz frames
  (`takeover::steps`), so no window is skipped and KH2's moves take the same time at either frame rate.
- **Pose once per game frame** (`kh2-rig` `RigAnim::update_deferred` / `settle`, `takeover`): a step before
  the frame's last keeps the animation's track and blend clock moving but doesn't evaluate the clip, blend or
  weapons; the last one (or `settle` after an early end of the steps) does. A step with a hit or guard window
  open still evaluates (hits read the Keyblade's place). `update` is `advance` + `render`, the same as before.
- **Effects take the frame's delta** (`kh2fx::tick(motion, delta)`, `takeover::physics`): one tick of `steps()`
  60 Hz frames instead of that many ticks of 1.0 (`kh2_fx::Effects::tick` adds the delta up as the game's
  effect clock does); KH2's frames tick after the steps, with the steps run.
- Nothing tested in game; the load at 30 fps should drop (one pose and one effect pass a frame instead of two).

### 2026-10-22: Stage 345 (testing branch: KH2's spells hit with the KH1 spell's own hit effect)
- The user: KH2's moves' hits only ever show the plain Keyblade effect. From the user's Ghidra export of the
  Steam exe (project notes `kh1-hit-effects.md`): KH1's hit spark comes from the attack object's effect list
  (`+8`), picked by the hit kind and the attack data's `+0x28` group; a swing's list is the equipped Keyblade's
  (`new_attack` on Sora), a KH1 spell shot's the spell's own. The spells' lists live in KH1's **magic slots**
  (`0x2D23580`, 0x38 each, one per KH1 spell: `+0` level, `+8` the loaded `.mag` file; the list at
  `file + [file + 8]`); the t341 run showed them staying loaded with no KH1 cast.
- `land_hit` for a spell (KH1 spell data): the KH1 spell's list from its slot goes in the attack object's `+8`
  (`pose::handle`, a pointer to a game handle) before KH1's contact: Fire's, Blizzard's and Stop's hit sparks
  (Thunder and Gravity have none, as in KH1). Slot not loaded: the Keyblade's, as before. Logged ("KH2 spell
  hit ...", 40 lines) and in the `hitfx:` trace note.
- `probe`: the t340-t344 investigation scans (actor / object table / data searches) are taken out again; the
  per-hit "hit effect:" log and effect-list dumps stay.

### 2026-10-22: Stage 344 (testing branch: the object-table scan takes any file that holds the list)
- The user's t343 run: the spells' list handles turn up only in passing places (two "last one" words at RVA
  `0x2D223E4` / `0x2DB4718`, the running effects' pool from `0x2D678B0` at 0x3C each, the attack objects from
  `0x2DB4718` at 0xC0 each), nothing that keeps them; KH1's actor loader (`0x140288460`) has other file
  layouts than the one the t342 scan checked (`+8` .. `+0xC` for some kinds, `0x140287E40`).
- `scan_object_tables` now reports any entry whose file (handle `+8` or slot buffer) holds the list within
  1 MB, with the list's offset in the file and the file's first 8 words (its section table).

### 2026-10-22: Stage 343 (testing branch: what holds the KH1 spells' effect lists)
- The user's t342 run (all of KH1's magic): Fire, Blizzard, Thunder, Gravity and Stop hit with their own
  lists (Gravity's has no kind-1 entry, Stop's normal hit is id 2), and no entry of the four object tables
  has a file whose effect section is one of them: they're loaded some other way.
- `probe::scan_refs` (Steam ranges): each such list's handle and address are searched for through the exe's
  data (RVA `0x4D5000 .. 0x2F131E8`) and every place logged as an RVA, to find what keeps them (and the code
  that loads them, from the exe).

### 2026-10-22: Stage 342 (testing branch: which KH1 object entry holds each spell's effect list)
- The user's t341 run: the KH1 spells' effect lists stay loaded and unchanged after Magic went to KH2 and a
  room change (same world), with no KH1 cast. They sit in a load arena (packed by load order), so they're
  found the way KH1 finds them: object entry -> model file -> effect section.
- `probe::scan_object_tables` (Steam only: the user's Epic exe in `D:\claude\EPIC EXES` is an older Epic
  release, contact at `0x2AF890` not `0x2B1ED0`, so the mod's Epic addresses for these tables aren't known):
  when a hit uses a list that isn't the Keyblade's, every entry of KH1's four object tables (types 1, 7, 8,
  12; Steam `0x14296B630`, `0x14298C6F0`, `0x14298C690`, `0x14298C750`) is checked: its file (handle `+8`,
  or its fixed slot buffer at Steam `0x140D2ADA0`) and that file's effect section; the entry whose section
  is the list is logged ("object tables:") with its id, type and slot.

### 2026-10-22: Stage 341 (testing branch: the KH1 spells' effect lists remembered by address)
- The user's t340 run: KH1's Fire, Blizzard and Thunder hit with lists at `0x...6c2b80`, `0x...6de500`,
  `0x...702d00`, the very addresses of the run before (a game restart apart), and no live actor in KH1's actor
  list has them as `+0x68` (the spell shots aren't in it). They're remembered by address now (logged with
  their RVA), so a KH2 spell's hit shows whether each list is still there, unchanged.

### 2026-10-22: Stage 340 (testing branch: where KH1's spells keep their effect lists)
- The user's t339 run: all six Keyblades share one weapon effect source, reloaded on equip (each list differs:
  Diamond Dust's sparks are ids 5..7, the others 0..2). From the Steam exe (project notes
  `kh1-hit-effects.md`): an actor's own effect source is `+0x68`, the effect section of its object's model
  file (section table `+0x10 .. +0x14`), set when it's made; KH1's spell shots put the spell actor's `+0x68` in
  their attack objects' `+8`. A KH2 spell could do the same with the KH1 spell object's file, if it's loaded.
- Log (`probe`): a hit with a list that isn't the Keyblade's finds the live actor whose `+0x68` it is, and logs
  its object entry (id, type, slot, file) and the file's effect section ("spell source:"); every KH2 spell hit
  logs those remembered spells' entries, files and lists as they are now ("spell sources (...)"), to see if
  they stay loaded without a KH1 cast (a room change).

### 2026-10-22: Stage 339 (testing branch: the hit-effect log per Keyblade)
- The user's t338 run (six Keyblades, KH1 Fire / Blizzard / Thunder, KH2 Fire / Blizzard / Thunder): KH1's
  spells hit with their own effect lists (Fire spark id 3, Blizzard 2, Thunder none of kind 1); KH2's spells
  with the Keyblade's (the bug); all six Keyblades showed one source address, so only Kingdom Key's list
  was logged. The log is now keyed by the list's contents too (a hash) and names the equipped weapon (save
  entry `+0x36`), to see each Keyblade's list.

### 2026-10-22: Stage 338 (testing branch: which KH1 hit effect each hit gets, logged)
- From the Steam exe (project notes `kh1-hit-effects.md`): KH1's contact `0x1402B1ED0` starts the hit
  effect with `0x1402B2190 (attack, kind, at)`: the source is the attack's weapon's effect list
  (`attack+8`, made from `actor+0x14C` -> `+0x18`: Sora's equipped Keyblade, so each Keyblade has its own
  sparks; a KH1 spell's own actor for its attacks), the entry the first whose kind mask holds the hit
  kind (1; 4 against a guard, looked up as 2; 0x40 when the record's `+0xC` has 0x20) and whose group
  mask holds `1 << (attack data +0x28)`; started at the hit point with `0x1402AC4C0`. Byte `+0xA8` is
  only the hit sound (bit `0x400000`). The user: Keyblades' hit effects do look different.
- `probe::hit` (any attack on anyone but Sora) works out that lookup: every hit's source, attack data
  group, kind and the spark id KH1 picks go to the trace ("hit effect:"); each new (source, group) and
  each new source's whole effect list (each entry's kind / group masks) to the log, to see which ids are
  hit sparks, Keyblade by Keyblade and for KH1's own spells.

### 2026-10-22: Stage 337 (testing branch: `hit_effect_test`, KH1's effects started on a KH2 hit)
- The user's t336 `hit_fx_test` runs (KH2 air combo, 12 values each): byte 0 of `+0xA8` is the hit
  sound (KH1 sound 11024 + the value), bytes 1 and 2 changed nothing heard or seen; no value of any
  byte changed the hit's look. KH1's contact plays one plain spark; a different look has to be
  started by us at the hit (the user's plan: per custom move, a KH1 effect or their own).
- Test (ini, off by default, read live): `hit_effect_test = ids` (ranges allowed, `0-20,30`) starts
  the next id, in turn, on each landed KH2 hit through `0x1402ABB60 (source, id, actor, 0x30)` from
  Sora's effect source (`+0x14C` -> `+0x18`), as `swing_start` does with a row's effect; on the foe
  hit (`hit_effect_on = target`) or on Sora (`= sora`). The id, actor, source and the call's return
  are logged and in the `hitfx:` trace note.

### 2026-10-22: Stage 336 (testing branch: the ini is read again whenever it is saved)
- The user: "can you make it so that the ini can be changed while the game is running?"
  `settings::get` checks the ini's modified time at most every half second and parses it again
  when it changed (logged: "settings reloaded"); the new settings replace the old (the old are
  leaked, as callers may still hold them). Settings only used at start (hooks put in, the
  abilities given, the trace mode's setup) still need a restart.

### 2026-10-22: Stage 335 (testing branch: `hit_fx_test`, which of KH1's hit effect bytes picks the hit effect)
- The user's t334 traces: a KH2 spell's attack object lacks bit `0x400000` (`000b5800`), swings
  have it (`00415809`, the contact adds `0x100000`). The swings' `+0xA8` bytes come only from the
  borrowed KH1 row: `[00, 05, 06]` on the swings, `[03, 05, 06]` on the air finisher (row 206). The
  user: the finisher's hit sound changed, its effect didn't. So is byte 0 the effect at all?
- Test (ini, off by default): `hit_fx_test = 0,1,2,...` writes the next value, in turn, into byte
  `hit_fx_byte` (0..2) of each landed KH2 hit's `+0xA8` bytes (the rest from the row), with bit
  `0x400000` set, spells too; logged and in the `hitfx:` trace note.

### 2026-10-22: Stage 334 (testing branch: the attack object's flag word in the hit-effect trace)
- The user's t333 trace (KH2 Blizzard, ATKP 248 sub 4, on an enemy): the spell hit was landed with
  KH1's Blizzard level 3 data (`0x6158`); the object's `+0xA8` held that data's own bytes
  `[03, 05, 06]` before and after the contact, yet the plain Keyblade hit effect played. Spells
  don't get bit `0x400000` on the object's flag word (swings do, as the Attack command's row does);
  is that bit what makes the contact use `+0xA8`? `hitfx:` now also shows the flag word before and
  after the contact (and whether `0x400000` is set), to compare a spell hit with a swing hit.

### 2026-10-22: Stage 333 (testing branch: trace notes for the hit effects of KH2's moves)
- The user: KH2's custom moves' hits only ever spawn the plain Keyblade swing effect. Every KH2 hit
  is landed (`land_hit`) through one of Sora's four KH1 rows (ground / air, swing / finisher), and
  that row's effect / sound bytes go to the attack object's `+0xA8`; the KH2 ATKP's own effect
  fields are not passed on. Trace notes (events column) to confirm before changing it:
  - `hitfx:` on every landed hit: the KH2 side (swing key or spell, ATKP id / sub, kind, element,
    reaction, effect_on_hit, damage_effect, random_effect, hit_sound, flags), the KH1 row picked
    (index, motion, data offset, fx bytes, trail / effect / voice), the attack object's
    `+0xA8..+0xAA` before and after KH1's contact, KH1's attack data words, target and damage.
  - `swing start fx:` the KH1 combo motion whose trail / effect / voice a KH2 swing starts with.

### 2026-10-22: Stage 332 (testing branch: a KH1 air swing out of any KH2 air state gets KH1's hop, not KH2's rise)
- The user (trace f8556, Explosion, then a jump and a swing on the jump's first frames): the swing
  sent him up. The swing came out of `combat::command`'s air command (KH2 had him in its Jump /
  Air), which passed `keep_rise = true` to `kh1_air_attack_started`, so KH2's jump rise (269 left)
  went on under the swing: 20.5/frame up, 184 up at the swing's 10th frame, 258 at its 17th. Stage
  313 had done this only for the swing out of KH2's jump arc on KH1's air states.
- The user's call, "grounded attacks only": no KH2 air state keeps its rise now (Jump, Air rising,
  Aerial Dodge, an air swing or cast): `keep_rise` is false at both callers; the faster sideways
  speed is still kept. Falling swings are as before (KH2's rise was 0 there).

### 2026-10-22: Stage 331 (testing branch: KH2's glide 1:1 with the Steam exe: the stored launch speed, no stick-scaled glide, no glide back to the hover)
- The user: "neutral flight doesn't store aerial dodges momentum, in kh2 ... if you aerial dodge go
  into neutral flight, neutral flight will store the speed from aerial dodge, so when you glide you
  go to max speed, or if you glide then let go you get a bunch of speed while your falling"; "it
  should be 1:1 so change everything that is currently different". (The rust rewrite's exe notes
  are the Epic build's; these are the Steam exe's, `KINGDOM HEARTS II FINAL MIX.exe`, found from the
  RTTI of `ACTION_GLIDE` / `ACTION_GLIDEHOVER`, vtables `0x1405CDBF0` / `0x1405CDB98`.)
- What the exe does: Sora's movement data keeps his velocity (`+0x20`) and a stored launch velocity
  (`+0xE0`, flag `+0xEC`). The jump's and the Aerial Dodge's launch (`0x1403CBB00`, the dodge from
  `0x1403CB9F0`: stick direction x the dodge's speed, 0 neutral) writes it and sets the flag; the
  air actions' shared enter (`0x1403CBD30`, jump, glide, hover) copies it back into his velocity
  while the flag is set; the hover's step (`0x140423650`) multiplies his velocity by 0.9
  (`0x1405AAA40`) and leaves `+0xE0` alone; the stick takes the hover into the glide
  (`0x140423840`), whose enter puts the launch speed back; the glide's step (`0x140423920`) is
  `(1 - a) * glide_speed + a * |v|` along his facing, no stick tilt in it; the glide's end into the
  fall (`0x1403CBC30`) clears the flag and keeps his velocity. The start picks the hover only with
  the stick neutral (`0x1403A8369`: action 0x11, else the glide, 0x12); the glide never goes back
  to the hover.
- `character-core`: `Character::launch_vel` (set at the jump's take-off and the Aerial Dodge, put back
  on `try_glide` and on the hover's turn into the glide, cleared at the glide's end and on landing);
  the glide's speed no longer follows the stick (stage 173 out), and a glide let go no longer drops
  into the hover once slow (stage 178's ours, `GLIDE_HOVER_SPEED`, out). Tests:
  `glide_in_place_hovers` (the glide flies on with the stick let go), new `hover_keeps_the_dodge_speed`.
- `takeover`: KH2's stored launch speed is kept when KH2 gives him back in the air (`LAUNCH`: from
  the glide's borrow, or a borrow that launched him), and the glide's borrow later in the same air
  time starts from it (an Aerial Dodge, KH1's fall, then Square: the dodge's speed comes back);
  without one (KH1's own jump) his speed at the borrow stands in for it. Cleared on the ground.

### 2026-10-22: Stage 330 (testing branch: the trace's camera columns compile)
- Stage 329 left `cam` out of the trace row `takeover::trace_frame` builds (E0063); it is
  `Default::default()` there, filled by `trace::frame` like `draw`.

### 2026-10-22: Stage 329 (testing branch: Sora's camera in the trace)
- The user: "add soras camera movements to the trace ... when I describe whats happening you can
  better understand and compare with the kh2 decomps camera and what we currently have"; the
  trace is the Select + Triangle recording.
- `camera.rs` notes each call of KH1's follow / lock-on / movement-state camera mode (`Seen`, read
  by `trace::frame` through `camera::seen`): who placed it (ours or KH1's, and why not ours:
  `kh1_chosen`, `event`, `kh1_only_state`, `scene`, or what `run` declined: `subject_not_sora`,
  `bad_feet`, `degenerate_camera`...), which entry (`follow`, `lock`, `follow<-stateN` for a
  movement-state camera ours took, `stateN` for one KH1 ran), the mode-entry flags, KH1's eye and
  look point (up = minus y), how far each moved since the call before, the eye's ground distance
  and the eye's and look point's height from Sora's feet, KH1's yaw / pitch / distance / field of
  view / blend, and KH2's camera's yaw, goal yaw, distance, the right and left sticks it read,
  reset, auto and the lock-on target's distance.
- 34 new columns `cam_*` / `kh2cam_*` before `events`. `cam_lag` is the game frames between the
  row and the camera's last run (1 is normal: Sora's physics runs before the camera); more means
  KH1 has the camera in another mode (`cam_mode_now`: event, area, conversation, first person).
- Not in: KH1's public eye / look point (`0x14299C4E0` / `4F0`, what is drawn, after a blend)
  aren't in the Steam address table yet.

### 2026-10-22: Stage 328 (testing branch: sound log names the SFX and voice bank files)
- The user: "we can do both ... if I say I want sora to say this line e.g 0x23 you can just cross examine what I said with the log"; "you can do both" (a sound-effect hook and a clip-to-file map).
- `soundlog.rs` (Steam build only for these): hooks the voice call (`0x1400DC210`, bank + clip no) and the effect create/play (`SslSeCreate`/`SslSePlay`). Voice lines log `voice call bank 93 no 37 = tw_sora.win32.scd clip 37`; effects log bank, no, id and the first bank-list nodes (dumped once, to decode the layout). `docs/kh1-sora-voice-banks.csv` maps Sora's bank/clip to SCD entries.

### 2026-10-22: Stage 327 (testing branch: KH2's glide on Square as the KH2 rewrite has it; KH2's air moves keep their input)
- The user: "it should behave exactly like the kh2 decomp ... in kh2 your allowed to glide out of
  air doge"; "glide should activate from pressing square, not jump". The KH2 rewrite binds the
  glide to the action button (`kh2-bevy` main.rs: "Glide on the action button ... jump still
  Aerial Dodges"; tests `glide_on_the_action_button`, `action_button_mid_dodge_glides`): in the
  air the jump is only the Aerial Dodge; Square glides whenever he is above the glide's start
  height, before or after the dodge or mid-dodge, held to keep it going.
- At Glide KH2 KH2's controller's glide button is Square (`GlideButton::Action`) whatever KH1's
  Superglide or the ini's `glide`. The borrow out of KH1's air (`shared::GLIDE`) is a fresh Square
  (up or down) with KH2's Glide equipped and high enough; its press is KH2's action press on the
  first frame (`ROLL_NOW`), and the Aerial Dodge is there in the borrow if it wasn't used this air
  time (`AIR_MOVES`); used there, it is noted at the hand-back (`shared::mark_air_dodged`) so
  KH1's air doesn't borrow him for a second one.
- The Aerial Dodge hide rule (stage 319: KH1 sees no jump until the dodge is used, then a fresh
  press) is KH1's glide's only: at Glide KH1.
- Stage 323's input fix back in: a KH2 move in the air out of KH1's states (`shared::kh2_move`,
  airborne, KH1 state 0, its "action running" bit up) isn't taken as KH1 holding him still, so the
  glide's held Square and the Aerial Dodge's stick and Square reach KH2's controller.
- Not done (the user: "everything but 4. for now"): hiding KH1's Square in the air at Glide KH2
  (KH1 has no Glide or Superglide there anyway, `glide_swap`).

### 2026-10-22: Stage 326 (testing branch: KH1's sounds in the log)
- The user: "we can do both ... if I say I want sora to say this line e.g 0x23 you can just cross
  examine what I said with the log". New `soundlog.rs` (ini `sound_log`, on by default; `off`
  hooks nothing) hooks KH1's menu sound (`0x140177F60 (id)`), voice group (`0x14028E060`) and
  voice clip (`0x14028E7A0`) and logs each call: `KH1 sound: menu sound 2 (from game +0x...)`,
  `voice group 4 on Sora (...)`, `voice clip 35 (0x23) priority 3 on Sora (...)`. A caller outside
  the game's code is the mod's own call (`kh1voice`, `trace`, `config`, `combat`). The prologues
  are the Steam exe's (read from it): `40 53 48 83 EC 30`, `48 89 5C 24 20`, `48 89 5C 24 08`;
  an Epic build with other bytes logs "not hooked" and goes on. `docs/kh1-sounds.md` is the table
  to fill in from the log; the log is capped at 6000 sound lines a session.

### 2026-10-22: Stage 325 (testing branch: the trace's sound is the menu's confirm)
- The user: stage 322's sound (`0x140177F60 (1)`) is the cursor's move sound; the one wanted is
  the confirm on a menu button. `trace::click` now plays sound 2 (the menu's "open" / on sound,
  `docs/kh1-config-menu.md`) when the trace is written, still 4 (the "can't") when it isn't. If 2 is
  not the confirm, 0 and 3 are the others to try.

### 2026-10-22: Stage 324 (testing branch: stage 323's two fixes taken back out, for now)
- The user: only the Glide ability swap was asked for; the two fixes are to be tested after it.
  Taken back out (as they were at t322): KH2's moves in the air out of KH1's states are again cut
  off from input while KH1's control step stops (`takeover`'s `borrow_air` gone), and at Glide KH2
  KH1's jump and Square are hidden in the air past the rise only (`shared::control`). Still open:
  every borrowed KH2 glide ends on its second frame (the t321 log), and Square on the way up can
  start KH1's own Superglide (less likely now: at Glide KH2 the shared list has no Superglide).

### 2026-10-22: Stage 323 (testing branch: KH2's own Glide ability by the Glide style; KH2's air borrows keep their input)
- The user: "if glide is set to kh2 both glide and super glide need to be removed from the shared
  abilities list and only the original kh2 growth glide should appear again as its own custom
  ability, if its kh1 then kh2s custom glide ability gets removed and it returns to vanilla".
  KH2's own Glide is ours, id 88 (`growth::ours::GLIDE`, `MENU`): shared list, no AP, made from
  KH1's Glide entry, its menu name with its level ("Glide LV2", "Glide MAX", `menu::set_glide_name`),
  `Effect::Kh2Glide`: LV1..LV3 by Sora's level (40, 52), MAX once Chernabog is beaten
  (`growth::kh2_glide_level`). It is never learned on its own: `growth::glide_swap` (each second,
  `menu::glide_style_list`) at Glide KH2 takes KH1's Glide and Superglide out of the shared list
  and puts KH2's Glide in the first one's place (on if either was on; a Glide or Superglide KH1
  gives later is taken out the same way); at Glide KH1 it takes KH2's Glide out and puts KH1's
  Glide back (on as it was) and Superglide too once Chernabog is beaten (on with Glide). Other
  ability modes put KH1's back before taking ours out. "Ability Up" boxes for the glide only for
  KH2's Glide now. With KH1's Superglide gone at Glide KH2, KH2's glide button is the jump.
  Tests `glide_swaps_with_the_style`, `kh2_glide_grows_to_max_at_chernabog`.
- The user: "currently unable to glide or sometimes can?" (t321 log: all 21 glide borrows ended on
  their second frame; trace f3281: KH1's own Superglide). (1) A borrow in the air sets KH1's
  state to 0, which queues KH1's landing and raises its "action running" bit, so KH1's control
  step stops through the whole KH2 move and `takeover` read it as KH1 holding him still: no
  input, so the glide button read as let go (and the Aerial Dodge had no input at all). Now a
  KH2 move in the air out of KH1's states (`shared::kh2_move`, airborne, KH1 state 0, bit 4 up)
  isn't taken as held still. (2) Square on the way up started KH1's own Superglide (the Glide KH2
  hide was past the rise only): KH1's Square bit is hidden all through the air at Glide KH2, its
  jump past the rise.

### 2026-10-22: Stage 322 (testing branch: a sound when a trace is recorded)
- The user: "can you make it so that when I record a trace a sound in game plays the sound when you
  click on something in the pause menu". Select + Triangle's dump (`trace::frame`) now plays KH1's
  menu click (`0x140177F60 (1)`, the sound the Config menu's toggles make) once the CSV is written,
  or the "can't" (4) if it couldn't be (the session's dump limit, a write error). `trace::dump`
  returns whether it wrote the file. The anomaly dumps, written on their own, stay silent.

### 2026-10-22: Stage 321 (testing branch: KH2's glide is KH2's controller's again; the glide off the physics setting)
- The user: "after incorporating kh2 glide as a shared ability it stopped it from being 1:1 with
  kh2's glide" (t320 traces, physics KH1, Glide KH2: KH1's glide state 0x15 on KH1's numbers with
  KH2's motion 205 drawn on it: KH1's +0.4 a frame speed ramp to 16, its 1.0..2.45 saw-tooth fall,
  no turn for a small tilt, and at release KH1's cut to 1.1 sideways with a hop up). Stage 284 had
  made the glide a shared move whose numbers followed the physics setting (`shared::physics`'s
  glide wrappers ran only at physics KH2). The user: option 2, "and not have glide be affected by
  the physics setting in the config menu".
- Glide at KH2 is a KH2 borrow again (`shared::GLIDE`): out of KH1's air, past the rise, the glide
  button (Square with Superglide or `glide = square`; else a fresh jump press once the Aerial
  Dodge isn't there for it: used this fall, or not equipped) with KH2's Glide equipped and above
  its start height (`takeover::kh2_glide_open`) borrows him; KH2's controller gets the press on
  its first frame (`JUMP_NOW` / `ROLL_NOW`, no Aerial Dodge in the borrow) and glides him with
  its own glide: speed, fall, turn, banking (206 / 207), hover (208) and its release into its
  fall, then KH1's fall when that is over (`give_back`). KH1 sees none of its glide inputs in the
  air past the rise at Glide KH2. KH2's controller keeps its Glide ability at Glide KH2 in shared
  states too (it was set to 0 there), so the glide button mid-Aerial Dodge glides straight away.
- Glide at KH1: KH1's own glide state and numbers at either physics setting. The glide wrappers
  on KH1's state 0x15 (`glide_pre` / `glide_step`) and `Numbers::glide` are gone; KH1's Square
  glide (`shared_frame`) only at Glide KH1.
- The glide's numbers never follow the physics setting now (KH2's controller's glide reads only
  KH2's growth table; physics KH1 only changes its fall after the glide is let go).

### 2026-10-22: Stage 320 (testing branch: Superglide on Square, glide mid-dodge, glide-hide logging)
- The user: "you can't superglide out of air dodge"; in vanilla KH1 Glide is the jump button and
  Superglide is Square. t319 had forced `glide_on_square` off, which took Superglide's Square away.
  Now the glide button follows the ability (`takeover::glide_on_square_now`: Superglide equipped, or
  the `glide_on_square` flag): `shared::control`'s hide (`Seen::superglide`), the Square glide in
  `shared_frame`, KH1's glide held while it glides (`gliding`), and the controller's `glide_button`.
- KH2's glide button (`character-core` `try_action_button`, `try_jump`, the AerialDodge action): a
  FRESH press of the glide button glides, also in the middle of the Aerial Dodge ("straight into
  Glide"); a held button never does. `shared_frame`'s Square glide is a fresh press now (a Square
  held through the dodge doesn't glide when KH1 has him back). `style_air_start` (KH1 glide with
  KH2 owning him) takes a fresh Square (Superglide) or a fresh jump press (Glide, when the dodge
  isn't there for it: `Character::air_dodge_available`), and in the dodge (`StateName::AerialDodge`)
  at once instead of only while falling.
- `kh2_air_dodge` keeps its last answer when the controller is busy (a failed `try_lock` read as
  "no dodge", so neither the borrow's hide nor the dodge hide held for that step).
- Logging for the dodge's first-frame glide request (still open): when KH1's step in
  `shared_control` asks 0x71..0x74, the log has the button words KH1 saw and the real ones, the
  hide, `AIR_DODGED` / `DODGE_JUMP_UP`, and the state flags.

### 2026-10-21: Stage 319 (testing branch: glide only after the Aerial Dodge, on the jump button)
- The user: KH1's glide request on the Aerial Dodge's first frame (t318 traces: KH1's air step asked
  its glide motion, 0x71 / 0x73, the frame KH2 borrowed him for the dodge) isn't wanted: "glide can
  be requested after aerial dodge only (if its equipped)", and "hold off on normal glide on
  square". `shared::control`: with the Aerial Dodge equipped, KH1 sees no jump in the air (past the
  rise) until the dodge is used, nor while the dodge's press is still held (`DODGE_JUMP_UP`): the
  glide, KH1's own held jump, needs a fresh press after the dodge. Without the dodge equipped
  nothing is hidden. Landing resets it (`AIR_DODGED`).
- Glide on Square is off: `settings::glide_on_square` is always false (the `glide = square` line
  is commented out, one line to bring back); the Square glide code (`takeover::shared_frame`,
  `glide_button`, `shared::control`'s hide) is untouched behind that flag. KH2's controller's own
  glide button is the jump again.

### 2026-10-21: Stage 318 (testing branch: the fall after an Aerial Dodge blends from the dodge's pose)
- The user: after an Aerial Dodge "goes back into the jump animation for a frame and soras feet
  seem to glitch out before snapping to his falling pose". The t317 draw columns: the dodge is
  drawn by KH2's controller (`draw_route` kh2_controller), then KH1's fall (slot 10, KH2 motion 4)
  starts with a blend from `Follow::last`, which only our clips set: the High Jump pose from before
  the dodge, 1.5-3 s old (blend 0.00 on the first frame, so the old jump pose showed). Now the
  controller's pose is stored in `Follow::last` each time it is drawn, and a clip blends only from
  a pose under 0.25 s old (as the slot path already did).
- `draw_age_ms` was the age at the time the trace was written; it is now the age when the frame
  was recorded.

### 2026-10-21: Stage 317 (testing branch: the trace records what is drawn on Sora)
- The user: asked for more in the trace to see Sora's model problems (the one-frame jump / glide
  pose at the start of an Aerial Dodge). `pose::wanted` now notes what it chose (`pose::drawn`):
  the new trace columns `draw_route` (kh1_own: KH1's animation, nothing of ours; kh2_controller:
  KH2's controller's pose; follow: a KH2 clip drawn for KH1's slot), `draw_slot` (the KH1 slot
  of that draw), `draw_play` (Motion / Swing / Slot, follow only), `draw_kh2_motion` (the KH2
  motion drawn), `draw_clip_frame` (frames into it), `draw_blend` (the blend into it, -1 none),
  `draw_restart` (the clip started this draw) and `draw_age_ms` (how long ago it was drawn: a
  large age means the row's draw is stale). The log also gets a "draw: ..." line whenever the
  route, slot or motion changes (up to 1500).
- Nothing about how he looks changes.

### 2026-10-21: Stage 316 (testing branch: Sora can act after an Aerial Dodge again)
- The user: "sora now gets locked out of moving after aerial dodge" (traces t315). Stage 315's
  fall request at the hand-back (`shared_give_back`) raised KH1's "action running" bit (bit 4 of
  the actor flags): `0x28007` for the whole fall and landing, where it was `0x28003` before.
  The bit is cleared right after the request, as the code does elsewhere for KH1's idle request.

### 2026-10-21: Stage 315 (testing branch: KH1's fall pose at the end of KH2's move in the air)
- The user: after an Aerial Dodge "sora's feet seem to glitch out before snapping to his falling
  pose". The traces (t314, Aerial Dodge out of KH1's and KH2's High Jump): KH1's motion request is
  0x07 (its landing, slot 11) all through the dodge and for the 1-2 frames after KH2's move ends,
  until KH1's air step asks for its fall, 0x06 (slot 10). The 0x07 is the landing KH1 queues when the
  borrow sets its state to 0 (`set(actor, 0)`). `shared_give_back` (Back::Air) now asks KH1's
  fall request (`0x14029D7F0 (actor, 6, 0)`) at the hand-back when the request is still 0x07.
- Line endings: the testing branch's files are CRLF; stages 313-314 wrote LF into `AGENTS.md`,
  `lib.rs`, `shared.rs`, `pose.rs` and `combat.rs`; put back to CRLF.
- Not done yet (the user: wait, KH1's glide logic first): the Aerial Dodge's first frame still
  shows KH1's last jump slot (12 / 13, now the High Jump clip) and the 0x71 glide slot for a frame.

### 2026-10-21: Stage 314 (testing branch: KH2's High Jump clip on KH1's High Jump)
- The user: "kh2 high jump is not using kh2s high jump animation". The traces (KH1 High Jump,
  height ~235: requests 0x9 / 0xa / 0xb / 0xc, slots 12 / 13 / 10 / 11; KH1 plain jump, height
  ~185: requests 0x4 / 0x5 / 0x6 / 0x7, slots 8 / 9 up) show the old comments had the pairs
  backwards (8 / 9 "high jump", 12 / 13 "jump"). `pose::play_for` now draws KH2's High Jump
  (motion 201) over slots 12 / 13 and the plain jump over 8 / 9; the fall loop takes over from
  either when KH2's arc falls, and a missing High Jump clip falls back to the plain jump.

### 2026-10-21: Stage 313 (testing branch: a KH1 air swing out of KH2's jump hops, it doesn't keep the jump's rise)
- The user's traces (physics KH2, KH1 air combo): a swing started just after KH2's jump took off
  kept the jump's remaining rise (`kh1phys::keep_momentum` takes KH2's arc when it rises more than
  the swing's hop), so Sora climbed from ~50 to ~214 while swinging; at physics KH1, and with
  KH2's own air swing (its hop replaces the jump's rise, ~+12), he doesn't.
  `kh1_air_attack_started` takes `keep_rise`: false for KH1's air attack out of KH2's jump arc on
  KH1's air state (`combat::command`'s first branch): the swing's own hop and arc, the faster
  sideways speed kept. True, as before, out of a KH2 move (an Aerial Dodge).

### 2026-10-21: Stage 312 (testing branch: KH2's air swings in KH2's order as KH1 slots)
- The `select_triangle` trace (t311): the three air swings played from slots 102 / 103 / 104, but
  stage 310 had built them as 183 / 184 / 181, not KH2's own air order. KH2's air combo is
  181 <-> 182 with 183 as the finisher (`docs/movement-model.md`, the stage 307 trace "Attack
  181 / 182 / 183"). `kh2slots::SPECS` and `pose::Play::Swing` now give 102 -> 181, 103 -> 182,
  104 -> 183.
- The "KH2 would take row ..." log line (`combomods::attack`) still shows 181 for both swings: it asks
  a trial copy of KH2's controller with no attack playing, so the anti-repeat step never fires.
  The log only; it doesn't choose the slots' motions.

### 2026-10-21: Stage 311 (testing branch: `kh1_slots`, on by default)
- The user: "can you change it to be kh1_slots? (kh2_slots sounds like kh2 is taking control, not
  kh1) and yes lets turn it on". The ini key `kh2_slots` is now `kh1_slots`, default `on` (a missing
  key is on; `off` gives the old borrow / draw-over path). An ini's old `kh2_slots` line is
  ignored. `Settings::kh1_slots`; the module is still `kh2slots.rs`.

### 2026-10-21: Stage 310 (testing branch: KH2's air swings as KH1 motion slots)
- The user: "can you add kh2s air attacks as slots in kh1?" `kh2slots.rs` (`kh2_slots = on`, or a
  list) now builds KH2's aerial combo as slots 122..124 after the ground ones, standing in for
  KH1's air combo (slots 102 / 103 / 104, requests 0xCC / 0xCD / 0xCE): 183, 184 and the
  finisher 181 (`pose::play_for`'s mapping). Tier A as for the ground combo: KH1's clock, hit /
  combo / trail windows, blends and sounds; KH2's motion drawn at KH1's time; KH2's sounds and
  effects from `kh2slots::tick`.
- With them built, `shared::kh1_combo(air)` is true at Aerial Base Combo KH2 too (`kh2slots::covers`):
  the Attack in the air stays in KH1's air state instead of borrowing him for KH2's air combo,
  so the borrow's hand-backs (stage 308's KH2 High Jump) don't apply to it. Hits, damage and
  lunge are KH1's (its attack rows), as at the ground combo.
- A request that isn't the slot a spec expects is now logged and that motion left out, not a
  failure of the whole table.
- Untested in game. Look for "KH2 slots: KH2 motion 183 / 184 / 181 as KH1 slot" in the log.

### 2026-10-21: Stage 309 (testing branch: stage 308 reverted)
- The user: "I'm still able to kh2 high jump, can you revert our changes" (trace t308: KH2's High
  Jump, motion 201, again out of KH2's air swing). `kh2_jump_open` is back to stage 307's (free
  states and command windows only; no trial step on a copy of KH2's controller).

### 2026-10-21: Stage 308 (testing branch: a jump out of the end of KH2's move is KH1's, again)
- The user: "do you know why I transition into kh2 high jump" (trace t305, Jump KH1, Physics
  KH2). A KH1 jump, an Attack in the air (Aerial Base Combo KH2: KH2 borrows him), KH2's air
  swing (motion 181) touching down on its last frame at f16228; the jump pressed at f16229 went to
  KH2's controller, which jumped him itself: KH2's High Jump (motion 201, his High Jump ability),
  not KH1's jump. No `Land` came between (the swing's end took the jump), so stage 307's landing
  hand-back didn't apply. `JUMP_OUT` (s261: a fresh ground jump where KH2's move can be left is
  KH1's own) asked `kh2_jump_open`, which knew only free states and command windows; the swing's
  end is neither, but KH2's own action takes a jump there (its cancel window, `f >= length`).
  `kh2_jump_open` now also tries the press on a copy of KH2's controller: if it would jump, the
  jump is KH1's. (The Aerial Dodge after it is by design: the jump press in the air is KH2's.)

### 2026-10-21: Stage 307 (testing branch: KH1 lands KH2's air moves in its own states)
- The user: KH1 should handle landing KH2's attacks with its own internal animation state instead of
  giving it back (the `select_triangle` trace: Attack 181 / 182 / 183 in the air, then KH2's `Land`
  for 26 frames with KH1 sitting in state 0 / motion 0 the whole time, then idle gave him back).
  `shared::give_back` now returns `Back::Land` the moment KH2's controller enters `Land` on the
  floor: `shared_give_back` sets KH1's fall state (2) with the mirrored momentum, as for `Back::Air`,
  and KH1's own state step lands him (its landing request and slot 11, its hold, its can-act
  rules), at either Jump style (KH2 draws its land over slot 11 at KH2). A command or press of KH2's
  still waiting keeps KH2's `Land` as before. KH1's own air casts landing into KH2's hold
  (stage 163, `land_now`) are unchanged.

### 2026-10-21: Stage 306 (testing branch: KH2's landing after an air move plays out)
- The user's trace (`select_triangle`): an air attack touched down into KH2's Land (motion 5), but
  `shared::give_back` counted `Land` as over (grounded, no combo) and handed him to KH1 two frames
  in, so the landing hold never showed. `Land` is out of the give-back; it ends into idle (or walk
  / run after the hold), and those give him back.

### 2026-10-20: Stage 295 (testing branch: KH2's physics in KH1's fields; KH2 animations as KH1 motion slots)
- The user: "kh2s physics gets written into kh1s internal physics system instead" (a KH1 air move
  out of a KH2 air move had no momentum). KH2's every 60 Hz step is mirrored into KH1's own
  velocity (`+0x3C0`) and arc (`+0x3B0..+0x3BC`) (`shared::physics::mirror`, `kh1phys.rs`); KH1
  taking him in the air starts from them; KH1's air attack out of KH2's motion keeps the faster
  sideways and the higher rise; KH2 taking him from KH1's air starts at KH1's speed. One switch
  point for the later setting (`shared::physics::source`, `Rule`). `docs/kh1-physics.md`.
- The user: "our kh2 animations can run entirely through kh1s animation system as new animation
  slots". KH1 resolves request -> slot (`+0x1D8`, 321) -> motion (`+0x1D4`, 118 handles); Sora gets
  longer copies (`docs/kh1-anim-slots.md`). KH2's ground combo (151 / 152 / 151 / 154) as KH1 slots
  118+ with KH2's windows as KH1 motion events, KH1's requests redirected at Base Combo KH2
  (`kh2_slots = on | <motions>`, default off; `kh2slots.rs`, `kh1_data::slot`). The pose is still
  drawn by us at KH1's clock.
- KH1's glide is slots 75..78 (requests 0x71..0x74 / 0x76..0x79), not 113..116 (attacks): the
  KH2 glide never showed at Glide KH2. Fixed.
# AGENTS.md

Notes for anyone (human or AI agent) working on this repo.

## Ground rules

- Never commit game files, extracted tables, dumps, Ghidra databases or decompiled code. The
  `.gitignore` is a whitelist: add a pattern only for source, docs or manifests.
- Put scratch output (table dumps, binaries) in `local/`. It is ignored.
- `combat-core` must stay free of KH2 knowledge and engine dependencies. KH2-specific
  mapping belongs in `kh2-data`.
- Write down every rule's source and confidence in `docs/combat-model.md`.
- Every change to the source gets a new stage: an entry in the development log below and
  `VERSION` in `crates/kh1-hook/src/lib.rs` bumped to match (the user: "every time a change is made
  to the source code make sure the version updates"). On the testing branch the version is
  `t<stage>` (`t300` on), not `s<stage>`; `tests/version.rs` checks it.
- When code is ported from another project, credit it in `NOTICE` and `CREDITS.md`
  and keep its licence.

## Several sessions at once

Up to one lead and three helper chats can work together. Follow `docs/WORKFLOW.md`: lanes
by crate, claims on the board in `local/board/`, delivery to the PC's `main` under a lock.

## Reverse-engineering notes

- Ghidra runs headless on a private copy of the user's executable, outside the repo. Commit
  only plain-language findings (in `docs/`) and our own Rust. Never commit decompiler output,
  Ghidra projects or byte dumps.

## Testing against the game

```
KH2_GAME="D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe" cargo test
```

## Development log

### 2026-10-21: Stage 305 (testing branch: KH2's jump hands over to its fall loop at the apex)
- The user: "letting go of jump still doesn't transition kh2 soras jump into his falling
  animation" (the "2nd animation" of stage 304 was the fall). Trace t304: released at f5680, the
  hop's apex 5 frames later, but KH1 kept its apex slot 9 (drawn as KH2's jump) to f5699 before its
  fall slot 10. KH2's own rule (`kh2_rig`): the jump plays while rising, then the fall loop. `pose`
  now shows KH2's fall (motion 4) on KH1's jump slots (8/9, 12/13) once KH2's arc on KH1's air
  state is past its apex (`shared::physics::kh2_arc_falling`, set in `air_step`), for every jump,
  a full one too. Stage 304's skip of the jump clip is taken out.

### 2026-10-21: Stage 304 (testing branch: a released jump shows KH2's jump clip's apex part)
- The user: "kh2 jump doesn't transition into its 2nd animation when you let go of the jump button,
  I only saw the 2nd animation when I did a max jump at the end". The trace (t303): the early
  release now cuts the rise to KH2's hop, but `pose` draws KH2's jump clip (motion 3, KH1's jump
  slots 8/9 and 12/13, one group) at its own clock from the takeoff: the hop's ~31 frames ended
  before the part a full jump shows round its apex (from about frame 28-33), then the fall loop.
  On the cut (`shared::physics::air_step`), `pose::jump_released` moves the clip on so the full
  jump's apex frame (3 frames of KH1's takeoff + its rise) comes with the hop's apex. KH2's own
  controller's jumps (`kh2_rig`, KH2 has him) are unchanged.

### 2026-10-21: Stage 303 (testing branch: KH2's early-release hop on KH1's jump, again)
- The user's trace (t302, physics KH2, Jump KH2): a standing High Jump on KH1's states with KH2's
  arc went to its full 235 though the jump button was let go 9 frames into the rise; KH2 cuts the
  rise to a hop there (`character_core::release_hop`, after 6 frames). The log showed `air_step`
  taking KH1's own rewrites of its arc fields during the rise (every other frame) as KH1 starting
  one of its own air moves: the arc was rebuilt from KH1's each time (`kh1_arc_on_kh2`), its time
  starting over and `holdable` lost, so the release check never passed. A plain jump's rise of ours
  (holdable, KH1's action flag 4 off) now keeps KH2's arc through those rewrites.
- Not a bug, from the same trace: the probe's "drawn at" y one frame behind `+0x490` is the
  frame-start copy (`takeover` writes it), the model is where the actor is.

### 2026-10-21: Stage 302 (testing branch: the trace clip's length is set in the ini)
- The user: "can you change it so that the csv clip can be adjusted in the ini file". New ini
  line `trace_seconds = 10` (1 to 120, 10 when missing; `settings.rs`): how many seconds of
  Sora's frames the trace keeps and writes on Select + Triangle or an anomaly (`trace.rs`; the
  row cap follows it at 120 rows a second). The anomaly cooldown stays 10 s.

### 2026-10-20: Stage 301 (testing branch: KH1's air attacks out of KH2's air moves get their lunge)
- The user's traces (t299): KH1's aerial combo swings start at a fixed sideways speed along his
  facing, 7.80 / 3.90 / 11.70 a frame for motions 0xCC / 0xCD / 0xCE, slowed x0.975 a frame (the
  finisher also rises 40); its finisher goes ~280 forward. Started out of KH2's air move (KH2's
  combo with the combo style KH1's), KH1's start left `+0x3C0` at 0: no sideways travel at all, he
  rose and fell where he was, under either Physics setting. The user: "lets give sora a fixed 11.7
  starting speed to match how the move would normally behave in a kh1 combo, this bug also appears
  any time a kh2 air attack transitions into kh1 attack". `shared::physics::seed_kh1_lunge`: after
  KH1's physics frame, within 4 frames of such a start (`combat::command`), the first frame of a
  KH1 aerial swing with no sideways speed gets that swing's lunge along his facing, decayed to the
  swing's frame (`+0x3B0`). It starts a frame late (its first frame's ~12 units are not made up).
- The trace's `kh2_*` columns are empty on frames KH2 doesn't have him (they showed KH2's last
  move, stale, all through KH1's).

### 2026-10-20: Stage 300 (testing branch: versions are `t<stage>`, bumped with every change)
- The user: "every time a change is made to the source code make sure the version updates, also
  instead of S for our lettering lets do T for test branch so now we should be at T300". `VERSION`
  is `t300`; the version test expects `t<latest stage>`; the rule is in Ground rules above.
- Stage 299's trace: the row built in `takeover::trace_frame` was missing the new `config` field
  (the build failed); it is set there (`trace::frame` fills it).

### 2026-10-20: Stage 299 (testing branch: Sora's per-frame trace)
- The user: "have the log keep track of kh1s soras position, kh2s soras position, the current
  anim state, etc... anything that would help you debug soras current actions and physics without
  me having to show you a video". `trace.rs`: a row for each of Sora's physics frames (who has him,
  where he is, how far he moved and how far KH2 asked, KH1's actor: state, motion, height above
  the floor, collision flags, velocity, arc; KH2's controller: state, motion, frame, air, on top,
  facing, combo, lock-on; Drive Form; the pad; the log lines of that frame). The last 10 s are kept
  in memory and written to `kh1-kh2-traces\` as a CSV on Select + Triangle (the user's pick; KH1
  doesn't see Select while Triangle is held, its first-person view) or, a second later, on their
  own when something looks wrong (a jump of more than 150 a 60 Hz step, 30 under the floor for 3
  frames, KH1 and KH2 trading him 8 times in 60 frames, a NaN). Ini `trace = on | always | off`
  (`always`: every row to `kh1-kh2.trace.csv` too).
- The user: "another important thing to add to the csv file is what settings are set in the config
  menu". Each row has the Config menu's KH1 / KH2 rows on that frame (Camera Style, Physics, every
  move style) and `physics_now`, whose numbers moved him (KH2's in a Drive Form).
- Every `kh1-kh2.log` line starts with the game frame it was written on (`[f123]`, the trace's
  `frame`), so the two line up.

### 2026-10-20: Stage 298 (testing branch: the Physics KH1 / KH2 setting)
- The user: "In the config there's not a setting to switch between KH1 and KH2 physics". Ini
  `physics = kh1 | kh2` (default kh2) and a Config row after Camera Style, live. Styles now pick
  only animations; `shared::physics::source()` picks the numbers (KH2 in a Drive Form). `kh2`: KH2's
  numbers on every base Sora move, KH1's own moves included; `kh1`: KH1's own states unmodded and
  KH2's controller given KH1's gravity / fall cap / jump / run / air steering (new
  `character-core` `air_accel`). The old hair switch `physics = on/off` is now `hair_physics`
  (an old ini's value carried over). KH2's air drift on KH1's states was applied twice as often
  as KH2 does; fixed. Choices for one-sided moves: `docs/kh1-physics.md` 7.
### 2026-10-07: milestone 1
- Surveyed existing work: OpenKH (format docs and C# readers), GovanifY/kh2 (PS2 decomp, early
  stage), KHPCPatchManager. Chose documented formats; no decompiling was needed.
- Ported the HED/PKG reader. The cipher's substitution table is located in the user's exe at
  runtime by a sliding-window permutation search, then checked against a SHA-256.
- Parsed 00battle.bin. The counts on the user's install match OpenKH's docs (atkp 2713, enmp
  229, ...). Live and backup copies are identical.
- Derived from the data:
  - ATKP power is a percentage (most common value 100; Thunder spells use 375 = ×3.75)
  - the LVPM level-99 Defense of 51 matches the community example
  - ENMP min/max damage × HP is nearly constant, which suggests a fraction of HP
- Open: the clamp reading, critical_adjust scope, element ids 4–6. Next: an MSET/ANB motion
  trigger reader to get hitbox timing and Sora's combo attack ids.

### 2026-10-07: milestone 2 (movement + combat playground)
- Added readers for 03system.bin `pref` (prty/fmab/plyr/sstm), PTYA combo rows, MSET/ANB motion
  headers, root-motion F-curves, motion triggers and model collision.
- Findings on the user's install:
  - trigger frames are 60 Hz and relative to the clip start
  - Sora's keyblade hitbox windows live in `W_EX010.mset` at the same slots as `P_EX100.mset`
  - root motion: Dodge Roll 438 units, A300 lunge 92
  - fmab records 0–3 = growth LV1–MAX
- Built `character-core` (engine-agnostic controller) and `kh2-bevy` (capsule playground).
  The playground was verified under Xvfb with `--synthetic --autoplay --screenshot N`. The
  real-data path was verified on the user's machine with `kh2-combat sora`.
- Open: gravity (estimated), the combo `sub` chain, skeleton-accurate hitboxes, enemy attacks.

### 2026-10-08: Phase 1 from the executable
- Ghidra 12.1 headless on the user's exe. MSVC RTTI names the action classes, so the
  action vtables (jump, glide, air slide, hop, idle) were easy to find.
- Implemented from plain-language notes:
  - the 30 Hz velocity tick
  - the double-sine jump with gravity 20/49
  - air control 0.9/0.95
  - glide, Quick Run and Aerial Dodge formulas
  - the PTYA selector (table order, `sub` anti-repeat, kind/flag semantics, `combo_offset`)
  - the hit-driven combo counter
  - jump-attack arcs and the lunge factor
  - the damage routine (stat source byte, base-term clamps, ceil rounding, difficulty table,
    boosts, Cure)
  - enemy status init
  - ATKP level variants
- Removed: estimated gravity, the approach-assist hack, the `--clamp` and `--power-scale`
  switches, and the score-based selection.
- Verified on the install:
  - the jump peaks at 185 on frame 33
  - whiffing alternates 151/153
  - connecting gives 151→153→155
- Open: jump hold, stick-tilt mapping, landing, cancel points, auto-target, timer units.

### 2026-10-08: skeleton, animation and a mannequin
- Aerial Dodge is the jump button in mid-air (KH2), then Glide; it was on the action button.
- New readers:
  - MDLX skeletons
  - full interpolated motions (initial pose, F-curves with pre/post behaviour, IK helpers,
    joint table, constraints, activations, limiters, expressions, external effectors)
  - `00objentry.bin`
  - `03system.bin` → `sklt` (weapon bones)
- New crate `kh2-anim`, implemented from plain-language notes on the exe's motion evaluator:
  - key times are 60 Hz
  - the unscaled/scaled matrix pair
  - constraints and limiters
  - one- and two-bone analytic IK
  - the expression language
- Root motion is now sampled at 60 Hz key time. Lunges were half length before: A300 now
  travels 150.
- Playground: Sora is a capsule mannequin on his real bones, playing his real motions, with
  the Keyblade attached to bone 178. Hits use the real bone-attached attack spheres.
  `--data-dir local/re` runs from extracted files (research only).
- Verified locally:
  - every Sora motion poses without NaNs
  - idle stands at y = 0
  - screenshots of idle, run, attack, jump, roll and glide look right
- Open: per-move blend times, foot ground adaptation, head look-at, the Shadow's own
  skeleton.

### 2026-10-08: playtest fixes
- Stick left/right was mirrored (screen-right is −X when looking along +Z).
- Read from the exe:
  - **Variable jump height:** an early release swaps the rest of the rise for a 5.1-unit
    hop, after 6 frames and before apex − 5.
  - **Quick Run** lasts only while the button is held, and is ground-only.
  - **Dodge Roll's** frame-10 event chains into Quick Run while the button is held.
- Air combos: whiffs now advance the count, per the user's knowledge of the game (the exe's
  rule for this isn't found yet; ground stays hit-only).
- Rig:
  - **End-of-action zoom fixed:** the last frame was posed as a free loop without the root
    offset, and blending then smeared that jump.
  - Bones are now kept in object space.
  - The Keyblade's blade/keychain orientation is fixed.
  - New simplified mannequin.

### 2026-10-09: Phase 2 (skeleton-accurate hitboxes)
- **Hit detection, from the exe:**
  - Attack windows collect the attack- and guard-type shapes of their collision group.
    Defenders use their hit-type shapes.
  - The overlap test: sphere, column, ellipsoid; cubes never hit.
  - The 1.5 × radius sweep between frames.
  - Implemented engine-agnostically as `combat_core::{Shape, AttackVolume, first_overlap}`,
    with unit tests.
- **Data:**
  - `kh2_data::sora::BoneShape` keeps every MDLX collision entry: type, kind, bone or root,
    offset, lift, radius, height.
  - Guard windows keep their group (`WindowKind::Guard { attack, group, weapon }`).
    `Output::guard_boxes` lists them.
- **Blending, from the exe:**
  - blend lengths of 8 (default and locomotion), 16 (glide and the fall helper), PTYA
    `blend_time` for attacks (0) and 0 for queued follow-ups
  - the clock starts at 2
  - walk↔run phase sync
- **Playground:**
  - hits use the game's test, with the sweep
  - F1 attack/guard, F2 hit, F3 skeleton lines, F4 other collision types
  - `--show-hurt`, `--show-skeleton` and `--show-other` for screenshots
- **Open:**
  - clash table
  - wall-hit ray
  - the motion freeze during a blend
  - group-enable bits from the collision header
  - enemy collision (Phase 3)

### 2026-10-09: playtest fixes, ledge grab, wall hits, Phase 3
- **Landing (exe):** landing hands over to the idle action. The stick takes over at once, and
  with a neutral stick Sora stops dead instead of sliding.
- **Arena:** a box top crossed during a fast fall now counts as ground, and the support and
  push-out radii match. This fixes slipping off box tops.
- **Ledge grab (exe):** the search box, timing and placement, using `hang_*` from prty/sstm
  and the type-4 radius. Motion 7 hangs; motion 8 climbs.
- **Wall hits (exe):** the BG-hit ATKP flag, the ray from the previous centre, the
  floor-check ray, and the recoil (motion 41/43, 50 push or hop). The swing counts as
  connected.
- **Reactions and clashes (exe):** the shared reaction routine (groups, motions, hit-back
  distance, launch physics), the 7×7 clash table with angle checks, and the party and enemy
  clash reactions.
- **Enemies:**
  - `kh2_data::enemy` loads the Shadow (M_EX010) from the install
  - `character_core::npc` runs it
  - the playground's Shadows attack on our timer, hit Sora and get guarded
- **Open:** enemy scripts, weight, revenge, Sora's damage-lock length.

### 2026-10-09: Phase 4 (HUD, menus, abilities, magic)
- **Playground changes:**
  - one Shadow by default (`--enemies N`)
  - the ledge hang waits for a fresh push or jump before climbing (our change)
  - Aerial Recovery (jump during a launch)
- **Enemy weight (exe):** the body push is split by objentry weight.
- **03system item/cmd tables and msg text:**
  - the command menu (Attack, Magic, Items, Drive)
  - the ability menu with real names and AP and the level's AP cap
  - HP/MP gauges
- **Correction (exe):** the ATKP row "level" byte is the defender's consecutive-hit count
  from the same attack group, not the attacker's level. It is now implemented with the
  game's per-object hit record (`combat_core::HitRecord`: hit-once bits, group/count, a
  30-frame group timer for the player, 4 re-hit interval slots). Sora's and the Shadows'
  swings use it too.
- **New crate `kh2-bdx`:** the BDX script machine, written from the interpreter. The trap
  names come from OpenKH's list, whose 828 argument counts match the executable.
- **New crate `kh2-magic`:**
  - spell casting (trigger 20 with caster, resources, sub and the `magi` record; trigger 21
    at the cast motion's magic-cast event)
  - the 59 traps the six spells use
  - spell attacks (the game's magic attack shape and sweep, duplicates sharing ids)
  - effects (PAX elements by id, bone codes, lifetimes from the DPD emitter headers)
  - a y-down/y-up flip at the boundary
- **Reaction 5 (exe):** Magnet's orbit pull.
- **Verified:**
  - every spell file at every level runs to the end headlessly (`kh2-magic` test with
    extracted files)
  - in the playground: Firaga hits 5 times (alternating 8/10-frame re-hits, a 175% final);
    Blizzaga's shard; Thundaga's bolt; Curaga heals; Magnega lifts the Shadow into an orbit;
    Reflega stops a swipe and counters with 574 ×5 and a 667 burst
- **Open:**
  - the MP Charge rate
  - the target search without lock-on
  - Drive Form variants
  - enemy scripts (the VM is ready)

### 2026-10-09: Phase 4 finished (map collision, camera, Drive Forms, reaction commands)
- **New crate `map-collision` (exe):** the game's background solver.
  - one sphere (r 90, centre 115) plus a lower sphere for walls
  - sub-stepped push-out: faces, then edges
  - anti-tunnelling segment test
  - the floor ray with a 24-unit step-down
  - the 49% ceiling rule with a 16-frame air-command lock
  - Step-up 25 and the 38.5° slope limit come from the sphere's geometry.
  - The arena is now polygons: stairs, ramps, a floating slab.
- **New crate `kh2-camera` (exe):**
  - the field camera: distance/height curve, swing-behind, spring smoothing, wall pull-in
  - the lock-on framing
  - target switching
- **Drive Forms (exe + data):**
  - each form's object from objentry/pref/PTYA/wmst, with two weapons where the data has them
  - `character_core::drive`: the gauges, costs, Anti roll, levels and growth
  - Sora's Drive motion and invulnerability, the arrival motion, revert, magic subs
  - Wisdom's shots from its model's BDX script (`Magic::object_effect`, trap
    `obj_effect_start`)
- **Reaction commands (exe + data):**
  - RC areas from type-5 collision with group switching (motion trigger 4)
  - priority/score selection, the low-HP Drive offer
  - capture motions read from nested motion sets
  - the Soldier's Cyclone, as its script runs it
- **Correction:** the "Shadow" was a Soldier (M_EX010). The real Shadow is M_EX020.
- **Open:**
  - enemy scripts
  - other enemies' RCs
  - items, form keyblade stats
  - the MP Charge rate

### 2026-10-12: KH1 port started (KH2 melee in Kingdom Hearts Final Mix PC)
- The user's request: KH1 stays KH1 except the melee combat, which becomes the KH2 rewrite's;
  KH2's abilities and combo modifiers go in KH1's Ability menu. The user's choices: KH1's own
  melee abilities are swapped for their closest KH2 counterparts in the same menu slot and
  level-up spot (the extra KH2 modifiers are added as new abilities); KH2 modifiers are learnt
  through KH1's level-up tables (Warrior / Guardian / Mystic) and equipped with AP.
- New crate `kh1-data`: KH1's packages (`kh1_first..fifth`, same format and cipher as KH2's,
  key table from the KH1 executable; `BackupImage/` originals first so a randomizer's edits
  don't leak in). CLI `kh1-extract --kh1 <install> --file a,b --out <dir>`. All 601 names in
  `kh1_first` resolve from OpenKH's `kh1idx.txt` / `kh1pc.txt` lists (the user's install).
- Sora is `xa_ex_0010.mdls` (8 sections; the first is `MOBJ`) and `xa_ex_0010.mset` (3
  sections; the first is `MMTN`, a motion table); battle table `btltbl.bin` (ability level
  tables at `+0x3BF8` and two more, per the Archipelago scripts).
- The KH1 executable is x64 and imports `DINPUT8.dll` (a proxy DLL; the
  folder is shared with KH2, BBS and Re:CoM, so the DLL must only act in KH1). The user runs
  OpenKH's Panacea (`DBGHELP.dll`) and LuaBackend with an Archipelago randomizer: stay
  compatible.
- Scope widened (the user, same day): KH2's movement abilities too, and KH1 Sora uses KH2's
  animations for everything except summons, Drive Forms (none in KH1) and magic. So KH2's
  controller drives Sora outright (idle, run, jump, glide, dodge, guard, hurt, attacks), as
  in KH2's combat mode, with KH1's collision underneath; KH1 keeps magic, summons, items,
  cutscenes and menus. Proposed (to confirm in play): KH1's story abilities map onto KH2
  growth (High Jump x1-3 -> High Jump LV1-3, Glide -> Glide LV1-2, Superglide -> Glide
  LV3/MAX, Dodge Roll -> Dodge Roll LV1); Quick Run, Aerial Dodge and the higher growth
  levels go into the level tables; Mermaid Kick and KH1's special attacks (Sonic Blade, Ars
  Arcanum, Ragnarok, Strike Raid) keep KH1's animations, having no KH2 counterpart.
- KH1 Sora's skeleton (`kh1_data::mdls`, read from the user's `xa_ex_0010.mdls`): 293 bones,
  0x30 bytes each (KH2's MDLX bone packed tighter: scale, Euler rotation, translation, links
  parent / sibling / child in 10-bit fields), bind pose a T-pose like KH2 Sora's (256 bones;
  KH1 Sora's hips at 78.9 against KH2's 88.5). `examples/skeletons.rs` prints both bind poses
  side by side for the retarget map.
- New crate `kh1-hook` (64-bit `dinput8.dll`, `-Zbuild-std` for `x86_64-pc-windows-gnu`):
  stage 1 only forwards DirectInput, and in KH1 alone logs a heartbeat to `kh1-kh2.log`
  (which other mods are loaded). Installed in the KH 1.5+2.5 folder.
- KH1's skeleton pass (Ghidra, the user's Epic exe; ASLR, so base + RVA): `0x140391E20`
  `(world, instance, time f32, motion)` samples the motion into per-bone arrays and builds
  the bone matrices (`0x140390260`); `0x140392710` `(world, instance, flags)` is the variant
  with ground IK (called by `0x14029E490` with the actor's instance at `actor+0x15C`). The
  instance: `+0` the model header (bone count, bone-table handle); handles to 0x40-byte
  per-bone matrix arrays at `+0x1B0..+0x1C0`, translation / rotation arrays at
  `+0x1C4` / `+0x1C8`. Game handles are 32-bit: `[exe+0x2EE4730][(h & 0x7FFFFFFF) >> 25] |
  (h & 0x1FFFFFF)` (`0x14038B230`). Matrices are D3D row-vector layout (translation at
  +0x30), i.e. glam's column-major memory.
- `kh1-hook` stage 2 (probe): inline hooks on both (x64: `jmp rel32` to a stub near the exe,
  trampoline with the moved prologue); logs Sora's (293 bones) arrays; Insert cycles a
  T-pose override through the five arrays to learn which one the game draws.

- Loading (the user: KH1 crashed launched directly; then the Epic launcher would not start
  with our `dinput8.dll` in the shared folder): launched without Epic's arguments KH1 crashes
  on its own (`-epiclocale` read as null in `0x1400D9BD0`). The proxy is out of the game
  folder. The DLL is now loaded by Panacea (OpenKH's `DBGHELP.dll`, read in Ghidra): in KH1
  only, it `LoadLibrary`s every `*.dll` in `<mod_path>\kh1\dll\` and calls `OnInit(wchar_t*
  mod folder)` / `OnFrame()` if exported (`kh1-hook` exports `OnInit`, which logs the route).
  Installed as an OpenKH mod (`crates/kh1-hook/openkh/mod.yml`: `dll/kh1-kh2.dll`, method
  copy) in the user's `mods/kh1/kh2-combat`, enabled in `mods-KH1.txt`, plus a copy in the
  built `mod/kh1/dll/` so it works before the next Mods Manager build.
- Probe fix (the user: Insert did nothing; log: loaded by Panacea, hooks in, but Sora never
  seen): the instance's `+0` points at a slot that points at the model header, whose bone
  count is a u16 (`0x140390260`: `**(short **)*instance`); we read one level short. Every
  model size posed is now logged once. `+0x1B0` holds the sampled channels (scale `+0`,
  rotation `+0x10`, translation `+0x20` per 0x40-byte bone), not matrices; `+0x1B8` the built
  matrices; the T-pose test only writes arrays whose bones look affine.
- Deploying `kh1-hook`: while KH1 runs, Panacea's copy in `mod/kh1/dll/` is locked and a
  write to it does not land (the upload reported success; the second test ran the old
  build). Rename the loaded file aside (`*.dll.loaded-old`, not loaded: Panacea takes
  `*.dll` only), copy the new one in, and check its SHA-256 on the PC after every deploy.
  The user's two crash dumps of 2026-10-13 (KH1 and the launcher, the launcher without our
  DLL) are both the null `memcpy` of a start without Epic's arguments.
- Stage 2 result (the user's Insert test, log): Sora is the 293-bone model posed by the plain
  pass (also by the ground-IK pass). Writing into `+0x1B4` gave a clean T-pose: that is the
  world matrix array the game draws (y-down world). `+0x1B0` holds the sampled channels (scale
  `+0`, accumulated scale `+0x10`, Euler `+0x20`, translation `+0x30` per 0x40-byte bone), kept
  between frames (writing it made him huge); `+0x1B8` is each bone's matrix relative to its
  parent (bone 3's matches its channels with KH2's Euler order). The T-pose lay sideways
  because it was placed from the bind's root rotation; in motion KH1's root is the hips with a
  yaw. Model → world is now `+0x1B4[0] * +0x1B8[0]⁻¹`, read each frame.
- New `kh1_data::retarget` (ours, from the two bind poses): KH2 Sora's pose on KH1 Sora's
  skeleton. Both bind in a T-pose facing +z with the same chain layout; 113 KH1 bones take
  their KH2 counterpart's model-space turn from rest (legs, spine, neck, head, arms, fingers),
  the rest turn with their parent; KH1's bone lengths, hips scaled by the hip-height ratio.
  `examples/retarget_check.rs` (KH2=1 for KH2's own pose) checked idle and run against KH2's
  own poses in side / front plots.
- `kh1-hook` stage 3: KH2 Sora loads on a thread from the KH2 install in the same folder
  (backup originals if the Mods Manager made them); Insert cycles off / KH1 T-pose upright /
  KH2 idle / KH2 run, written into `+0x1B4` after the game's pass (the Keyblade still follows
  KH1's hand). Logs a check that `+0x1B8` chains to `+0x1B4`.
- Waist fix (the user's screenshot of the in-game idle: the waist contorted). KH2's root,
  pelvis, spine, first hip bone, ankles and arm bones carry another built-in orientation in
  its motions than in its bind pose (the idle's pelvis 210° from rest, ankles ~115° off their
  foot every frame), so copying their turns from rest spun KH1's belt, pouches and shorts.
  `kh1_data::retarget` now measures every turn from KH2's joint positions: torso frames from
  bones riding on each part, limb segments aimed joint to joint and rolled by the knee /
  elbow direction (foot / knuckle line while nearly straight), fingers by segment.
  `examples/roll_probe.rs` checks it (leg rolls within ~30° of the foot in the idle; the run's
  kicked-back thigh 69° from the pelvis where the foot-rolled version gave 166°).
- KH1 motions (`kh1_data::mset`, from the user's `xa_ex_0010.mset` and the skeleton pass
  `0x140391E20` / channel evaluator `0x14038F9D0`): the `MMTN` table (118 slots for Sora),
  per motion frames / fps / bones / channels / keys, keys `time<<8 | interpolation` (1/256
  frames; hold, linear, Hermite with per-frame tangents). KH1 animates bodies through ~11 IK
  helpers per motion (ground root, body centre, hips with their yaw, pelvis, chest, hands,
  feet; records of scale / parent / rotation / translation) and per-bone flags pick the IK
  chains and their target helpers, so plain channel playback leaves the legs straight. Slots
  read from the helper paths: 0-1 standing, 2-4 walking (?), 5-7 running (looping), 8-10
  jumping / in the air, 11 landing; to be confirmed in play. A motion's counts (frames,
  channels, helper channels, helpers) name its slot (116 signatures for 118 slots; the shared
  pairs 77/78 and 86/87 are not movement).
- `kh1-hook` stage 4: the motion playing is at `instance + 8`; standing / walking / running /
  in-air / landing slots get KH2's idle / walk / run / jump or fall / land (8-frame blends),
  every other slot keeps KH1's animation. Insert cycles follow / off / T-pose / idle / run.
  Slot changes are logged with speed and climb.
- Stage 5 (the user: a split-second vanish going idle → walk/run; requests: jump, glide and
  attack animations, Keyblade and no-Keyblade versions in their situations). Play log
  (2026-10-13): slots 0 / 2 / 5 field idle / walk / run, 1 / 3 / 6 the battle ones (the field
  ones follow attacks back to the field); jump 12 → 13 → 10 → 11; glide 102 / 103 / 104;
  ground combo 98 → 99 → 98 → 101. `pose::play_for` maps them: field slots take KH2's field
  variants (no Keyblade; KH2 has them for idle / walk / run), jump / fall / land, glide / glide
  end, and swings 151 / 152 alternating with finisher 154 (KH2's no-target ground rows),
  each stretched to the KH1 swing's length and with KH2's lunge taken out (KH1 moves him).
  The vanish: some passes leave `+0x1B4[0]` as we wrote it, and model → world read from it
  compounded our own pose (the log's speeds of 10^4-10^5 units/s too); the last placement
  is kept when the root reads back as ours. The Keyblade: a small model (≤ 40 bones) whose
  world matrix holds a fixed relation to KH1's hand over 20 frames while the hand moves is
  taken as held and re-placed from the hand drawn (logged; lets go if the relation breaks).
  `examples/kh1_motions.rs` / `kh2_rows.rs` list both games' motions (scratch research).
- Stage 6 (the user: the vanish still there; animations jumbled; air combos glide; sliding
  in every animation). Log: 102 / 103 / 104 are the air combo (two swings and a finisher),
  not the glide (the user has no glide yet); now KH2's 183 / 184 alternating and 181. The
  vanish: speeds of 10^4 units/s still at every slot change, so KH1's blends between motions
  mix our drawn pose back into `+0x1B4[0]`; placement now comes from the pass's own world
  matrix (y and z columns negated, axes normalised, as `0x140391E20` does with `× -1` at
  `0x140430938`) once it agrees with the bone-derived one for 30 frames (logged either way),
  and meanwhile a jump of over 30 units in one frame is ignored unless it persists 3 frames.
  Sliding: walk / run play at his measured speed over the loop's ground speed (planted heel
  against bone 0: KH2 run 484 units/s, walk 111, field run 427, field walk 134; KH2's own run
  speed 8 / frame = 480 / s agrees), scaled by the hip ratio. Swings still slide with KH1's
  own lunge until KH2's controller moves him. Combo alternation no longer resets on the idle
  KH1 plays between swings (1.2 s gap).
- Direction change (the user, 2026-10-13: "I want the EXACT KH2 combat, not just
  animations"; sliding stayed because KH1 still moved him). KH2's controller
  (`character-core`) will drive Sora: input, movement, physics,
  jump / glide / dodge / guard, the combo system and hit timing; KH1 keeps its world,
  collision, enemies, HP / MP, magic, items, summons, menus. The animation-follow mode
  (`pose::follow`) stays as the fallback until then. Stage 6 log: placement from the pass's
  matrix took; glide is slots 75 / 76 / 77; 117 unknown.
- Ghidra findings for the takeover:
  - Input: every controller type ends in `0x140112C70` `(device state, PADDATA out, config)`,
    KH1's PS2-style pad record: `+0` buttons u16 in PS2 order (bit 0 Select ... 4-7 d-pad
    Up / Right / Down / Left, 8-11 L2 R2 L1 R1, 12-15 Triangle Circle Cross Square; it clears
    opposite d-pad pairs), `+2..+5` stick bytes (order to confirm), `+0x1F` status. XInput is
    imported by ordinal (2 GetState at `0x140113B80`), DualShock / DualSense through HID
    (`0x1401132A0`, `0x140111490`).
  - Floor query `0x1401910E0` `(query)`: `{x, y, z, 1, 0, range, 0, 1}` in, nearest level
    surface vertically within range at `+0x20` (point), `+0x30` face (short, -1 none)
    (`0x140190B00` walks the cells' polygons); used by the ground-IK pass.
  - Actor animation update `0x14029CF10` `(actor, dt)`: `+0x10` position (vec4, to confirm),
    `+0xD0` world matrix, `+0x15C` animation instance handle, `+0x168` motion slot u16,
    `+0x16C` motion time, `+0x1D4` motion table handle.
- `kh1-hook` stage 7 (probe, `probe.rs`): finds Sora's actor through his instance, searches it
  for his drawn position, puts hardware write watchpoints (debug registers on every other
  thread, a vectored handler) on that field for 25 s and logs the writing code by address;
  logs the pad record on change.
- Stage 7 probe result (the user's run): Sora's actor at `0x142D37C80` (party array, Sora
  first; actors are 0x4B0-byte records in a pool from `0x142D37CA0`); position `+0x10`
  (copies at `+0x50`, `+0x100` in the world matrix at `+0xD0`, `+0x260`, `+0x2E0`, `+0x320`);
  the only regular writer is `0x140386D00` (vector add) from the collision mover
  `0x140299350`, ~57 times a second. Pad record: Cross 0x4000, Circle 0x2000, Square 0x8000,
  d-pad as PS2; stick bytes rx, ry, lx, ly (left: 0 = left / up, 0xFF = right / down).
- Ghidra, the actor's frame (`0x140295E50`): think (`0x1402957F0` picks it by object kind;
  `0x1402A1170` for the player and party: the move vector, the state's handler from the
  table `0x142DB4240` (0x40 bytes a state: flags, enter, exit, update, displacement,
  transition, ...; registered by `0x1402AF9F0` from `0x1402AF8B0`), the turn toward `+0x150`),
  physics `0x140298230` (state displacement, platform carry `+0x120`, `0x140297710` move with
  collision, floor `+0x320` / height above it `+0x3D4`, transition), matrix, animation.
  `0x14298D3A0` is set while an event runs. The state at `+0x70` stays 0 through KH1's
  attacks, so KH1's own actions are told by the motion slot instead.
- `kh1-hook` stage 8 (`takeover.rs`): KH2's controller drives Sora's movement. Hooked
  physics step; for Sora, with no event and KH1 showing standing / walking / running (slots
  0-7) or KH2 having him in the air: input (direction from KH1's camera-relative move vector
  `+0x20`, tilt from the left stick, Circle jump, Square KH2's action button), KH2's step
  with KH1's height above floor, KH2's displacement through KH1's mover `0x140297710`
  (stepping down up to 24 onto lower floor on the ground, put back on the floor when below
  it), facing written to `+0x30..+0x3C` and `+0x150`, KH2's animation player for the pose
  (field variants on KH1's field slots). KH1 doesn't see Circle / Square while KH2 has him.
  Every movement ability at MAX for testing. Insert: KH2 controls / KH2 animations on KH1 /
  off / T-pose / idle / run. Cross still goes to KH1 (talk, chests, menus, KH1's attacks,
  which hand control back to KH1 until they end); KH2's combo system is next.
- Stage 8 result (the user + log): KH2 drives walk, run, dodge roll, quick run, aerial dodge
  and glide (300-500 units up off ledges); the jump played but didn't rise: KH1's mover snaps
  a grounded actor back onto the floor each frame, undoing the jump's first small rises.
  Fix (8b): only the sideways move goes through the mover; in the air (or rising) his height
  is KH2's, landing on the floor the mover reports (`+0x324`); on the ground the mover's
  height stands, stepping down onto a floor up to 24 below, else he falls. `+0x3D4` is
  rewritten for KH2's next step. The user's next requirement: attacks 1:1 with KH2.
- Phase 3 (combat parity) started; the user: movement looks great. Ghidra:
  - KH1's hit pipeline: `0x1402BD180` `(attack, target)` (an attack touched an actor: checks
    the target can be hit `0x14028FEE0`, queues a hit record, MP for the attacker from the
    attack data `+0x2C`, the attack's hit script) → `0x1402BD480` `(attack, hit-once bit,
    target)` takes a 0x40-byte record from the pools at `0x142DE6810` / `0x142DE7810` and
    links it on the target (`+0x3EC`, hit-once bits `+0x3E0`): attacker `+4`, damage `+8`,
    flags `+0xC`, knock-back direction `+0x10`, attack data `+0x2C`; the target's think pops
    it (`0x1402BCFB0`) and applies damage and reaction.
  - KH1's damage `0x1402BD8D0` `(attack, target, flags)`: power = attack data `+0x14` +
    attack `+0x2C`, critical (chance `+0x94`, bonus `+0x98`), against the target's defence
    (stats `+0x6C` → `+0x50`) through the table `0x142DE67C0`, min / max from the attack data
    (`+0x20` / `+0x24`) scaled by attack `+0xAC`, clamps from the target's stats (`+0xC0` /
    `+0xC2`), element resistance (stats `+0x78 + element * 4`, element = attack `+0x30` bits
    6-9). Attack objects: owner handle `+0x34`, attack data handle `+0x5C`, hit-once bit
    `+0x54`.
  - KH1's collision pass `0x1402B1A50` tests 0x40-byte entries (owner, attack object,
    shape at `+0x20`, overlap `0x1402B3D50`); attack vs body → `0x1402B1ED0` →
    `0x1402BD180`; attack vs attack → `0x1402B2A30` (clash, priority `+0x68`).
  - Battle mode: bit 0 of `0x142867CD8` (picks field / battle idle-walk-run in
    `0x1402C9FF0`). Actor iterator `0x14028F990` `(previous) -> next`. Actor kind byte `+6`
    (3 enemy AI, 4 / 5 / 9 / 10 objects), body radius `[[+0x134] +0x14] +0xC`.
- `kh1-hook` stage 9 (combat probe): set-motion hooks log the code putting Sora into
  non-movement slots (stack return addresses), the attack-touched-actor hook dumps Sora's
  attack records / targets' stats and keeps the last as a template, the actors are listed
  every 4 s. Fixed before shipping: `0x14029D630`'s third argument is a float (XMM2).
- `kh1-hook` stage 10 (combat parity, `win/combat.rs`), from the stage 9 log and Ghidra:
  - The Attack command `0x1402A5380` (command type 1 in `0x142D54440`) runs a row of the
    actor's attack table with `0x1402A4760` `(actor, row, dir)`; `0x1402A17F0` reads the row
    (`0x142D5D4B0[object id] + row * 0x14`: motion request `+0`, attack data
    `0x142D23740 + [+4]`, effect / sound bytes `+0xC..+0xE` → attack `+0xA8..+0xAA`, flag
    `0x400000`) and makes the attack object with `0x1402B10E0` `(actor, hit bit from
    0x1402B15D0, data)` (pool `0x142DB5110`, 0x40 × 0xC0; freed by `0x1402B13D0`).
  - Per frame `0x1402B22D0` builds the body entries: hittable actors (`0x14028FEE0`), team
    `(+0x374 >> 5) & 3`, parts at `[[+0x134] +0x18]` (0x14 bytes each, count byte `+6`,
    disabled bits `+0x3A8`), shape by `0x1402B3CB0` (type, radius, half height, centre `+0x10`).
    Contact `0x1402B1ED0` `(attack entry rcx, body entry rdx)` does the record, its push
    direction, the hit effect and the Keyblade hit sound (`0x140293FD0`).
  - While KH2 has Sora, a type-1 Attack command is swallowed and becomes KH2's attack press.
    KH2's swept attack shapes and hit records  test KH1's body parts;
    each effective hit is landed by a fresh attack object from Sora's own row (ground / air ×
    swing / finisher = KH1 motions 200 / 202 / 203 / 205), hit-once bit cleared during the
    call, then freed. `probe::hit` scales the record's damage by the KH2 row's power.
- `kh1-hook` stage 11: the user's stage 10 log: KH2's hits land (31-72 on the ground); the
  Attack command's air rows are 204 / 205 (203's first row, 0x4C58, hit for 95-144). Area
  scripts (`0x14028C550`, per-actor action list at `+0x440`, pool `0x142BA1E80`, 0x60 each,
  kind byte `+0x10`, next `+0x58`) walk Sora through doors / zone entries: KH2 lets go and
  `pose` shows KH1's own animation while the list is non-empty. Diagnostics for the
  Keyblade (invisible during KH2's attacks): its instance is combat state `+0x18`; where it
  is posed is logged with each KH2 state change.
- `kh1-hook` stage 12: the user: the game crashed going through a door (stage 11; no dump,
  last line KH2 retaking Sora between the door's turn and its walk, actor flags `0x1000000`
  set). Actor flag `0x1000000` (script commands 0x24 set / 0x23 clear) now also keeps KH2
  off (set from a door's turn to the end of its walk, and at a zone entry). `win/crash.rs`
  logs the first fatal exceptions (code, address, registers, return addresses). The
  Keyblade instance is never posed by the two skeleton passes; its own matrices are logged.
- `kh1-hook` stage 13: the crash note found the door crash: ours, in the stage 9 probe's
  set-motion logging (`probe::callers` scanned 0x800 bytes above its frame and ran off the
  top of the game thread's stack when a door script set Sora's exit motion). The scan now
  stops at an unreadable page. The Keyblade object (combat state `+0x18`) is no model
  instance (`[0x82, 0, 0, 9, handle, ...]`); its model instance is the handle at `+0x10`.
- `kh1-hook` stage 14: the user: doors work; the Keyblade is still invisible while
  attacking. Ruled out offline (`kh1-data` example `hand_check`): no KH2 attack motion
  degenerates or stretches the retargeted right hand or the 40 bones under it (101-127,
  280-292). The combat state's `+0x18` object is no model either: it is a table (count `+0xC`,
  0x20-byte entries from `+0x10`, looked up by `0x1401DDE80`), the hit-effect points. Probe:
  every small model (≤ 40 bones) posed near Sora is logged with where its placement matrix
  comes from (a bone of Sora's drawn matrices, if it points into them), at each KH2 state
  change and every 15 frames of an attack.
- `kh1-hook` stage 15: the stage 14 log: no small model is posed near Sora's hand at any
  time (none of the passes poses a Keyblade model). KH1 Sora's own skeleton carries the
  weapon: bone 280 under the right hand (grip, 4 units from it), 281-282 at the same point,
  283-292 a chain of short segments (flags 1) reaching 56 units (`kh1-data` example
  `bones_under`). The probe now logs bones 280 / 283 as KH1 posed them and as we drew them
  (position, determinant, distance from the hand). The weapon-object probe is gone.
- `kh1-hook` stage 16: the user's video (Traverse Town, no enemies): Sora's hands are always
  empty; the weapon bones are drawn in his hand (stage 15 log), so KH1 hides the Keyblade
  itself: KH1 shows it only in its battle motions (field idle / walk / run 0 / 2 / 5 carry
  none; KH1 brings it out for attacks). KH2's field / battle stance now follows KH1's battle
  mode (not KH1's slot), and while KH2's pose has the Keyblade out (`weapons_out`: battle,
  or an attack in the field) Sora's actor is kept in KH1's battle stance (`+0x374` bit
  `0x40000`, which `0x14029E050` uses to map a request to the field or battle slot) with
  the idle request replayed (`0x14029D630(actor, 0, 0.0)`); back to field when it is put
  away. Finished actions blend into idle over 14 frames (`RigAnim::queued_blend`).
- `kh1-hook` stage 17: stage 16's idle request never changed Sora's slot (stayed 0, redone
  every frame): `0x14029E050` maps a request to a slot through the motion table, and the
  stance bit only applies to actors with flag `0x4000` or no combat state, so not to Sora.
  Request bit `0x20000000` passes a slot through unmapped: the battle idle (slot 1) is now
  set directly while KH2 has the Keyblade out, the field idle (0) when it is put away.
  (The user: fetch `kh1-kh2.log` from the PC directly, never ask for it.)
- `kh1-hook` stage 18: stage 17's raw battle-idle slot took, but KH1 put the field slots
  back every frame and the Keyblade stayed hidden. The real switch (Ghidra): applying a
  motion (`0x14029DAB0`) draws the weapon for table entries of type 1 (`0x1402A0CA0(actor,
  1)`: actor flag `0x4000`, combat state `+0xC..+0xF` = ff ff ff 80) and puts it away for
  type 0 (`0x1402A10F0`: clears the flag); the flag also maps requests to their battle
  variants. The flag now follows KH2's pose (`weapons_out`). After an attack in the field,
  KH2 keeps its battle stance for 6 s (the user: combos ended in the empty-handed idle).
- `kh1-hook` stage 19 (retarget polish, the user's video: the Keyblade shows now):
  - Trouser leg snapping in falls: the thigh's roll flipped 180° within a half frame in
    falls, landings, jumps and several combo swings (`kh1-data` example `snap_check`). The
    knee's direction came from the knee's offset from the hip-ankle line, which lies along a
    raised thigh; it is now the hinge plane's (`bend`: thigh × shin), with the hips' forward
    swung along with the leg as the straight-leg fallback (`Pole::Knee`). Elbows use the same
    `bend`. Worst half-frame thigh turn: 180° → under 50°.
  - The shorts' four panel roots per leg (hip + 10 / 14 / 18 / 22) turn halfway between
    pelvis and thigh.
  - Shoes: the foot's roll is the model's up while the foot lies flat (`Pole::Up`), the shin
    only as it stands up, so soles stay flat when KH2's shins lean out.
  - The hood (root 79) turns with the chest's yaw and 30% of its lean; every bone no part
    turns now hangs as in the first frame of KH1's battle idle (slot 1, `set_rest`) instead
    of the T-pose (hood, belt chain, cloth joints, face, the Keyblade's grip and chain).
- `kh1-hook` stage 20 (the user's video of stage 19: the belt chain floated out in a loop,
  shins showed through the shorts in the battle idle, odd thumbs in the field idle, the hood
  could be lower, Sora briefly invisible, feet clip stair steps):
  - KH1 rest pose only for the hood's cloth joints (80-84) and the Keyblade (280-292); the
    belt chain (10-15) and the rest keep the bind (the battle idle's chain swings out).
  - Shorts panels follow the thigh again (the snap was the thigh roll, fixed in stage 19).
  - Hood root: chest yaw only, tipped down 0.45 rad (tip 45° below the neck).
  - Thumbs mapped (KH1 207-211 / 103-107 on KH2's thumb segment 89→92 / 181→184).
  - The skeleton pass's world matrix, once confirmed, places every later Sora at once (it
    was re-learnt per zone, placing him by his bones meanwhile: the brief invisibility).
  - Open: feet on stairs (KH1's ground IK is replaced by our pose; needs our own foot IK).
- `kh1-hook` stage 21 (the user: thumbs still odd, feet through stairs, the chain through
  the shorts, fit the jacket's lower part to KH2's motions):
  - Thumbs (103-107 / 207-211) take KH1's own pose: the battle idle's grip while KH2 has the
    Keyblade out (`takeover::GRIP`), the field idle's (slot 0) open hand otherwise
    (`Retarget::set_rest_open`, `pose_grip`). The KH2 thumb mapping is gone.
  - Belt chain root (10) turns 80% with the +x thigh; the jacket's back hem (72) 35% with the
    chest, the rest with the hips.
  - `win/feet.rs`: foot IK while KH2 has Sora on the ground. KH1's floor ray `0x1401910E0`
    (the ground-IK pass's query: start `+0`, 1.0 at `+0xC`, length `+0x14`, 1.0 at `+0x1C`; hit
    `+0x20..+0x28`, hit id i16 `+0x30`, -1 none; KH1 world y-down) under each sole (bones
    22 / 48); the body drops for a planted foot's lower floor (≤ 30), each leg bends (thigh
    17 / 43, knee 18 / 44, ankle 19 / 45) to put a planted sole on its floor or lift a raised
    one out of a step; eased (35% a frame).
- `kh1-hook` stage 22 (the user: the chain still sits oddly, the thumbs never changed, the
  jacket must mesh with KH2's motions). Measured offline before shipping (`kh1-data`
  examples `thumb_check`, `clip_check`, `kh1_follow`, `snap_check`):
  - Thumbs: KH1's own motions leave the thumbs at their bind (so no KH1 rest pose changed
    them). They are now aimed along KH2's thumb (89→92 / 181→184) with the part's rest frame
    measured on KH1's bind (`layout`'s `kh1_rest`): KH1's thumb lies exactly as KH2's
    (0° apart in every motion checked).
  - KH1 animates its cloth (belt chain, hem) by hand per motion (no blend of hips and thighs
    fits it: 20-140° residuals), so: the belt chain (10) hangs with the hips' yaw only; the
    jacket hem's nine chains (72; 248-276) turn with the hips (KH2 parents its own hem to the
    pelvis), front panels a little with their thigh; and `keep_clear` swings any cloth chain
    about its root so no joint comes nearer a thigh (hip → knee) than in the bind (≤ 14).
    Hanging and pushed cloth move at most 0.35 rad a frame (no snaps when the hips' yaw
    flips in somersaults).
- `kh1-hook` stage 23 (the user: in the battle idle the chain clips through the shorts and
  the hood through Sora):
  - The belt chain (10-15) is carried rigidly by the +x thigh (43) as in the bind
    (`ATTACHED`, `attach`): its distance to the outer shorts panel (57→59) is the bind's in
    every motion checked (a free-hanging chain fell inside the shorts when the battle
    stance spread the legs).
  - The hood chain (79: 80-84) is kept off the back (71→78) and the head (78→151) by
    `keep_clear` (up to 30), so the hunched battle stance can't tip the back into it.
- `kh1-hook` stage 24 (the user's stills: the back of the chain lies in the back of the
  shorts in every stance; the hood clips wildly in the battle idle):
  - Hood: no more hanging or pushing. Root and joints (79-84) take KH1's battle-idle relation
    to the neck base (78, on the chest part), so the hood lies on the back as in KH1 and
    moves with the chest (it can't meet the back). Tip 2-8 above the neck in KH2's stances.
  - Chain: still carried by the +x thigh; its lower joints (14, 15) also kept 4 further off
    that thigh than in the bind (constant: 13.8 / 20.1 from the thigh axis in every motion
    checked). The chain mesh's back part is presumably skinned with the hips (we don't read
    KH1's skin weights yet); if it still clips, reading the mdls mesh weights is next.
- `kh1-hook` stage 25 (the user: the hood is at 90° again; leave the chain; shoes spin round
  in the glide):
  - Hood: still carried by the chest in KH1's battle-idle relation, now folded down 0.7 rad
    about the level axis square to it (`HOOD_FOLD` in `set_rest`): tip 7 below the neck
    base, its joints 7-13 off the spine axis, constant in every motion.
  - Feet: `Pole::Foot` replaces the up rule: the foot rolls with the shin's hinge axis (the
    shin part's x, frames now computed in order, `frames`), turning to the model's up only
    while the foot is flat and its ankle low (below 24, none above 40, KH2 units). The up
    rule alone had turned the glide's pointed, sole-up feet over. `foot_check`: glide
    (205-207) shoes within 9° of KH2's feet.
- `kh1-hook` stage 26 (phase 4, combat parity, finishing):
  - Lock-on: KH1's lock state `0x142DB3F10` (non-zero while R1 holds a target) and target
    handle `0x142DB3F18` (`0x1402912F0`, the command menu, aims every command at it). KH2's
    controller locks the same enemy (`Character::set_lock`), so swings home in on it.
  - Finishers land with a KH1 finishing row whose attack data differs from the swing's
    (KH1's first 202 row shares the swing's data); every distinct attack data's first words
    are logged for picking knockback by its fields next.
- `kh1-hook` stage 27 (the user: enemies like the Large Body must repel hits from the front;
  some enemies' parts take no damage and bounce the Keyblade off):
  - KH1's guards are attack objects of priority 100 (`+0x68`; the clash `0x1402B2A30` repels
    anything meeting one), directional when type bits `0xE0000` are 1 or 3 (`0x1402B24B0`:
    Sora within the arc whose cosine is the data's `+0x64` around the owner's facing turned
    by the data's `+0x60`). `combat::guards` scans the attack pool (`0x142DB5110`, 0x40 ×
    0xC0; in use `0x800`, active `0xC`, motion-bound type 2 only while the owner plays it)
    each frame; a KH2 swing meeting one is repelled (`Character::register_wall_hit`, KH2's
    repel) and hits nothing more, with KH1's clash spark (`0x1402AC4C0(*0x142D67400, 10,
    20000, pos)`). Guards seen are logged once per owner.
  - Finishers: KH1's combo finishers are motions 203 / 206 with reaction-kind-5 data
    (0x4C58; the data's third word is the reaction kind, 3 a flinch, 5 knock-back).
  - Lock-on confirmed in the user's log (KH2 locks KH1's R1 target).
- `kh1-hook` stage 28 (Phase 5, taking hits): `win/hurt.rs` hooks KH1's hit pass `0x140299A40`.
  Before it runs for Sora under KH2, each ready hit record (queue at actor `+0x3EC`, next at
  `+0x30`, delay bits 12..15 of `+0xC`) is decided with KH2's last frame: invulnerable
  (dodge roll) → evaded, guard state / guard windows with the attacker in front (Guard Plus:
  anywhere) → KH2 guard repel + KH1 clash spark; both mark the record no-effect (`+0` bit
  0x20, as KH1's own guard does). Taken hits keep KH1's damage pass (Second Chance, MP,
  HP bar, dying) with the reaction kind cleared, then KH2 plays the reaction (3 / 4 flinch,
  5 blow) and the hurt voice `0x14028E060(actor, 0|1, 0)`, rumble `0x14028D740`, journal
  count `0x14028A4D0(0xD)` and command clear KH1 would have. Damage types 9..12 (drains,
  heals) are left to KH1. HP gone / stats flag 4: `takeover` releases Sora to KH1.
- `kh1-hook` stage 29 (Phase 6 / 7 groundwork):
  - Platform carry and push: KH1's physics step adds, before its move, the carrying
    platform's motion (`+0x200` while `+0x160` bit 4) and the frame's push (`+0x120`, zeroed
    after), and clears the touched-objects mask (`+0x90`). KH2's moves now do the same
    (`takeover::carried`, logged the first 40 times it is non-zero). Before, `+0x120` was never
    cleared while KH2 had Sora.
  - KH1's movement state (`+0x70`) is in the hand-off logs (for Phase 6/8 hand-offs).
  - Phase 7: `growth.rs` (unit tested) maps KH1's menu to KH2's abilities: save block
    `0x142DE9D60` (`*0x1428695A0`), Sora's level byte `+4`, abilities `+0x44` (48 bytes,
    bit 7 = not equipped, 0 ends the list as `0x140282A70` counts), shared abilities `+0x599`.
    KH1's own abilities map directly (High Jump levels, Glide + Superglide, Guard, Dodge Roll,
    Combo Plus, Air Combo Plus, the FM modifiers to KH2's nearest); KH2-only ones come with
    Sora's level (`LEVELS`). `win/menu.rs` reads `kh1-kh2.ini` (`abilities = test | kh1 |
    all`, default `test`, the old testing set) and logs what KH1's menu gives every time it
    changes.
- `kh1-hook` stage 30: `win/settings.rs` reads `kh1-kh2.ini` once (written with defaults when
  missing): `abilities` (test | kh1 | all), `hurt` (kh2 | kh1), `carry` (on | off),
  `ceilings` (on | off), so each feature not yet tried in game can be switched off without a
  build. Ceilings: while KH2's displacement rises, the move goes through KH1's mover in 3D
  with Sora's movement state set to 5 (falling) for the call only: `0x140297F20` builds the
  collision query's mode from the state (air mode 0x10 vs ground 4 for the player's team;
  the ground mode is what snapped rises back to the floor in stage 8). Less than 49% of the
  rise done: KH2's `hit_ceiling` (logged with the collision flags `+0x2D0`; bit 3 is what
  KH1's own jump `0x1402AD260` treats as a ceiling).
- `kh1-hook` stage 31 (Phase 6, KH1-only moves): KH1's jump runs, in its per-state input
  step (`0x1402ADA90`, the state table's third handler), checks that hand Sora to KH1's own
  air moves: `0x1402C5710` vines / climbable surfaces (state 10, collision id `+0x2DE`),
  `0x1402C68A0` ledges (state 13, from the collision's contact `+0x2F0` / `+0x300` and the
  stick), `0x1402C7E00` bars (skipped while `0x14298D3A8`), `0x1402C17B0` hang points on
  other actors (body part type 3), `0x1402BF410` swimming (state 17, water level `+0x2D4`).
  While KH2 has him falling (Air / Jump, stick held) they are tried in the same order (ini
  `grabs`); one that takes him sets `HANDOFF`: KH1 keeps him until its state is 0 / 1 and its
  motion slot a movement one again, then KH2's controller starts over standing (`FRESH`).
- `kh1-hook` stage 32 (review fixes for 28–31): KH1's fall state 3 (not 5) for KH2's rises
  and the grab checks: its collision has flag 1 (no floor snap) and mode 4 like KH1's own jump,
  its input step has the grab checks, its wall-solver extra only clears `+0x3C4`. Grabs refresh
  KH1's contacts with a standing-still move in that state first and keep the state only if a
  check took him. `HANDOFF` lasts at least 10 frames. Any release marks `FRESH` (KH2 restarts
  standing when it has him again), and the in-air exception uses `airborne` only while KH2
  has him. `hurt`: KH2 reacts only to kinds 3–5 (others stay KH1's), never restarts a blow nor
  a flinch with a flinch, plays KH2's recoil (not its guard) for attack clashes 6 / 7, and
  passes the pass's rax back. Rises take KH1's mover's height (`y = got`).
- `kh1-hook` stage 33 (Phase 8, KH1's commands stay KH1's): hook on KH1's command step
  `0x1402A5380 (actor, dir)` (called from the think step `0x1402A4090` while a command is
  pending at `0x142D54440`). Attack (type 1) still becomes KH2's attack via `exec`; types
  that only touch the party or the menu (`0x1402A1470`'s switch: 0x1D 0x1E 0x46 0x51 0x54
  0x64 → `+0x98`'s actor; 0x4B.. 0x60 → menu) pass; any other (magic is 2, spell in bits
  8..23) hands Sora to KH1 (`takeover::hand_to_kh1`, the `HANDOFF` rules). KH1 picks the
  command's version by its movement state (0 ground → `0x1402A1C20`; 2 jump → the air cast);
  in KH2's air he is first put into KH1's jump state falling from where he is, as KH1's
  ceiling bonk does (`0x1402AF7D0(actor, 2)`, `+0x3B0` / `+0x3B8` / `+0x3BC` 0, `+0x3B4` his
  y, `+0x3C0` velocity 0, `+0x3D0` 0, flag 0x10000 off). Ini `commands`.
- `kh1-hook` stage 34 (Phase 9 feel): KH1's combo row (0x14 bytes) also carries what its
  motion's start plays (`0x1402A17F0`): +2 the weapon trail (`0x1402C0160(actor, n)` asks for
  it in the combat state, `+0x38` = n | 0x40000000; `0x1402BFD70` makes it while actor
  `+0x1CC` bit 0x800000, which the attack motions' triggers set over the swing, and ends it
  when the bit clears), +3 an effect on Sora (`0x1402ABB60(*(cs+0x18), n, actor, 0x30)`),
  +0xF a voice (`0x14028E060(actor, n, 0)`). Each KH2 attack motion start plays those of the
  KH1 row of the same place in KH1's combo (200 / 201 / 202, finisher 203; air 204 / 205,
  finisher 206), and the trail bit follows KH2's hit windows (ini `swing_fx`; whether the
  trail was made is logged for the first swings).
- `kh1-hook` stage 35 (review fixes for 33–34): the trail flag is cleared whenever KH1 has
  Sora (any physics frame KH2 doesn't take, and before a KH1 command); a command hands Sora to
  KH1 only if KH1 changed his state or motion request (`+0x164`), else the in-air jump state
  is undone and KH2 keeps him; `exec` and `command` act only while KH2 has him and he is not
  handed to KH1 (`takeover::kh2_has`); a stale Attack press is dropped when KH2 starts over;
  a repel's recoil (KH2 motions 41 / 43) is not a swing start; the swing rows are cached per
  model and never empty.
- `kh1-hook` stage 36 (Phase 7, KH2's abilities in KH1's menu): KH1's ability table is
  `btltbl.bin`'s (loaded to `0x142D23740` by `0x14028D840`), entries of 12 bytes at
  `*0x142D37748` (static `0x142D26B18`), 65 of them: +0 AP, +1 kind, +2 menu order (u16; the
  menu sorts lists by it, `0x1402836C0`), +4 name handle, +8 help handle (from the remaster's
  `AbilityName.bin` / `AbilityHelp.bin`, made handles with `0x14038B080`, which registers any
  pointer's 32 MB window in the handle table `0x142EE4730`). `0x14028D760(id)` = table +
  (id - 1) * 12 is every reader (menu name `0x1402E5D40`, help `0x1402E5D10`, AP used
  `0x1402E62D0`, the message placeholder in `0x1401746D0`). `win/menu.rs` copies it into an
  80-entry table with ids 66..77 (template entry, KH2 AP, our text in KH1's codes), points
  `0x142D37748` at it (again after `0x14028D840`, hooked), and in `abilities = kh1` adds them
  to Sora's lists at their level (`growth::MENU`; shared ones on, the rest off); other modes
  take them back out. Final Mix's Combo Master (65) maps to KH2's. Commands now count as
  KH1's when the script-held flag or the motion slot changes too (spells set the flag).
- `kh1-hook` stage 37 (the user: no animation for a jump cast; the Square shortcut didn't cast;
  check moves don't repeat):
  - A command counts as KH1's when KH1 sets any motion on Sora during it (`probe`'s
    set-motion hooks count them: `SORA_MOTIONS`); the air cast (`0x14029D7F0(actor, 0x83 +
    spell)`) changed none of state / request / flag / slot at that point, so KH2 kept him and
    hid the cast's motion.
  - KH1's shortcuts are L1 + a face button: with L1 held, Circle and Square reach KH1 and are
    not KH2's jump / action (a button held past L1's release stays KH1's until let go).
  - `kh1-data/examples/combo_mix.rs` runs KH2's controller with the user's menu (Combo Plus x4,
    Air Combo Plus x2, Combo Master, the nine mapped modifiers) against near / far /
    surrounding / raised foes: no move follows itself; ground combos mix 151 / 152 / 153 with
    Slapshot, Flash Step, Slide Dash, Vicinity Break and finish with Guard Break / 155 / 154,
    air combos alternate 181 / 182 (as KH2 does) and finish with Aerial Finish.
  - A full ability list (48, KH1's most) is reported once per ability, not every second.
- `kh1-hook` stage 38 (the user: KH2 abilities have priority): when Sora's list (48) is full,
  `growth::make_room` picks a KH1 ability that is off to give up its place: first a spare copy
  of one he has two or more of (most copies first), then, in `MAKE_ROOM` order, Encounter
  Plus, EXP Zero, Cheer, Scan, Evolution, Jackpot, Lucky Strike, Treasure Magnet, Second Wind,
  Tech Boost, Leaf Bracer, MP Haste, MP Rage, Berserk, Critical Plus, Second Chance. Never an
  equipped one, ours, Guard, Dodge Roll, the specials or the modifiers. Each removal is logged
  and appended to `kh1-kh2-removed.txt` next to the game (unit-tested on the user's list).
- `kh1-hook` stage 39 (the user: keep Encounter Plus; Magnet Burst doesn't pull; where is
  Explosion?):
  - Encounter Plus is off `MAKE_ROOM`; abilities `kh1-kh2-removed.txt` lists that may no
    longer be given up are given back once a run (off), another making room if needed.
  - Magnet (ATKP reaction category 5, Magnet Burst): an enemy it strikes is caught (orbit
    radius between the attack's two knockback values, swirl its next value) and, while the
    swing lasts, drawn to the pulling hitbox's centre with the exe's suction step (toward the
    radius at up to 20 a frame plus the swirl, as `character_core::npc`) through KH1's per-frame
    push `+0x120` (its physics step adds it to the move and clears it, enemies included).
  - KH1 abilities that are KH2 moves here show their KH2 name and help in the menu
    (`growth::RENAMED`: Vortex → Flash Step, Counterattack → Counterguard, Blitz → Finishing
    Leap, Sliding Dash → Slide Dash, Hurricane Blast → Explosion, Ripple Drive → Vicinity
    Break, Stun Impact → Guard Break, Gravity Break → Aerial Finish); ids unchanged.
- `kh1-hook` stage 40 (the user: Magnet Burst's and Explosion's exact KH2 sounds and effects?):
  `win/kh2sound.rs`, the KH2 sound player (`docs/kh2-sound.md`): once KH2 Sora is loaded,
  the sounds of the effect elements the chosen motions start (`EventKind::Effect`, PAX "p_ex"
  in `obj/P_EX100.a.us`) are read from the user's KH2 install (`se/seNNN.win32.scd`, Microsoft
  ADPCM) and played through `waveOut` when KH2's clip starts the element. Ini `kh2_sounds`
  (`magnet_explosion` default: 166, 197; `all`; `off`), `kh2_volume`. KH2's visual effects
  (PAX/DPX particle systems) are not drawn yet: that needs our own particle renderer inside
  KH1's Direct3D 11 frame.
- `kh1-hook` stage 41 (the user: the KH2 sounds don't sound right; dig into the KH2 install):
  `docs/kh2-sound.md` "Sound ids are id records": `Scd::sound_id` resolves a sound id through
  the bank's id records (it was played as a stream number: wrong sounds), with
  their volume; Ogg Vorbis voice streams decode (`lewton`). `EventKind::Sound` (frame trigger
  8, weapon bank 500 = the Kingdom Key's) and `EventKind::Voice` (13 / 14, VTBL) join the
  effect elements; `kh2sound` plays all three, and KH1's swing voice stays quiet on those
  moves. Research tools: `kh1-data/examples/move_sounds.rs` (renders a move's KH2 timeline to
  WAV), `atkp_sounds.rs`; the static Linux CLI runs on the user's PC VM
  (`local/tools/kh2-combat-linux-3 extract --backup --game ...`).
- `kh1-hook` stage 42 (the user: every KH2 sound is right but the voice; keep KH1's voice):
  `kh2_voice` in `kh1-kh2.ini`, off by default: `kh2sound` loads no VTBL voices and
  `covers()` is false, so KH1's swing voice plays on Magnet Burst and Explosion with KH2's
  sounds and effect elements. `kh2_voice = on` brings KH2's battle voice back.
- `kh1-hook` stage 43 (the user: give Sora more AP to make up for the KH2 abilities): KH1's
  party stats `0x1402A2FA0` `(member, out)` fill `0x142E94BC0 + member * 0x100` from the save
  (max AP `+0xE4` = member `+9` plus the accessories' `+4`, at most 100; `0x1402E6220` is AP
  left = that minus the equipped abilities' AP, `0x1402E6120` reads it). `win/menu.rs` hooks
  it: Sora's max AP gets `growth::bonus_ap(level)`, the AP of every `growth::MENU` ability
  learned by then (27 by 50). Nothing is written to the save. `bonus_ap` in the ini.
- `kh1-hook` stage 44 (the user: KH2's real visual effects for Magnet Burst and Explosion):
  - `docs/kh2-effects.md` + `kh2_formats::dpd`: KH2's effect machine (PAX elements, 30 Hz
    effect ticks, emitters / programs / keys, MegaBirth sprites, ribbons, models, textures
    with the palette swizzle, `effect/texcommon.dpd` for VRAM addresses below 0x3000).
  - New crate `kh2-fx`: plays elements and outputs world-space triangles per batch (texture,
    GS blend, depth test, depth offset); `examples/fx_render.rs` renders them to PNGs.
  - `docs/kh1-render.md`: KH1 renders with **Direct3D 12** (D3D11 only decodes movies). Renderer
    `0x1422BB580` (device `+0x2E0`, frame buffers, constant shadows), View `0x142E9DE60`, P at
    the projection struct `+0x80`, KH1's clip formula, the world-model pass `0x1401EADD0`.
  - `win/fx12.rs`: our D3D12 drawing recorded into KH1's current list after the world pass, on
    its bound frame buffer (RTV / DSV by id, its MSAA count), own root signature (40 root
    constants + a root SRV raw buffer of RGBA8 textures, bilinear wrap in the shader) and
    PSOs per blend mode; KH1's root signature, heaps and dirty state restored with
    `0x1401087B0` / `0x140102980` (KH1 re-binds topology and vertex buffer per draw itself,
    `0x1400FBA20`). `win/kh2fx.rs`: loads `P_EX100.a.us` + texcommon from the KH2 install,
    starts elements on `EventKind::Effect`, ends motion-bound ones on a motion change, steps
    per KH2 controller frame, bones from `pose::kh2_pose`. Ini `kh2_effects`, `fx_test`.
- `kh1-hook` stage 45 (the user: the game crashed at Explosion's effect): KH1's current list is
  usually a **bundle** (`0x1400FE6E0` creates its allocator and list with type 1;
  `0x1400F7290` closes it and runs it with `ExecuteBundle` in the direct list at command
  buffer `+8`). Our `OMSetRenderTargets` is not allowed in a bundle, so its `Close` failed
  (`E_INVALIDARG`) and KH1's `ExecuteBundle` of the broken bundle crashed. `fx12` now checks
  the list type (`GetType`) and sets targets only in a direct list (a bundle inherits them),
  then hands KH1 a fresh list with its own mid-scene sequence (`0x1400F7300(R, 0)`,
  `0x140109360(R, 1, 0)`, as in `0x1400FBBA0`) instead of `0x1401087B0` + `0x140102980`.
- `kh1-hook` stage 46 (the user: casting Thunder crashed the game): the hit pass `0x140299A40`
  takes **two** arguments, `(actor, out vector)` (rdx, kept in r13 and passed to the
  reactions as r8; the knock-back reaction `0x14029AD30` writes the actor's position there).
  `win/hurt.rs` hooked it as `(actor)` and called KH1's with a garbage rdx; harmless while
  KH2 had Sora (KH1's reaction kind cleared), it crashed when a hit landed while KH1 had him
  (a spell cast). The hook now takes and passes both.
- `kh1-hook` stage 47 (the user's ability levels): High Jump (one) = High Jump MAX; Glide =
  Glide LV3, Superglide = Glide MAX; Quick Run and Aerial Dodge LV3 when learned (their level
  growth dropped; the dodge roll keeps its own). Sora keeps one High Jump / Glide / Superglide
  (`growth::one_only`, an equipped copy kept): spares are taken out of the shared list in
  `abilities = kh1` and noted in `kh1-kh2-removed.txt` (without "(id", so they are not given
  back).
- `kh1-hook` stage 48 (the user): High Jump is High Jump LV3 (not MAX).
- `kh1-hook` stage 49 (hook audit, after stage 46's crash): for every hooked function, the
  argument registers it reads before writing (rcx / rdx / r8 / r9, xmm0..3) and whether its
  callers use rax / xmm0 after the call (a Ghidra `CallUse.java` over every call site), and the
  same for every KH1 function we call. Found: `0x14029D7F0` and `0x14029D8A0` (set motion)
  take a float third argument in XMM2 (stored at actor `+0x184`) and their result is used by
  callers; `0x140392710` (skeleton pass with ground IK) takes a float third argument. The hooks
  declared those as integers, so KH1 got whatever XMM2 held: now `f32`. Everything else matches
  (`0x1402A4760` 3 ints, `0x1402A5380` 2, `0x140299A40` 2 (46), `0x1401EADD0` /
  `0x14028D840` none, `0x1402A2FA0` 2, `0x140391E20` (int, int, float, int), `0x14029CF10`
  (int, float), `0x140112C70` 3, `0x14029D630` (int, int, float), `0x1402BD180` 2 → used
  result, `0x140298230` 1).
- `kh1-hook` stage 50 (the user: softlocked in the Dive to the Heart's attack tutorial; the
  Dream Sword held backwards; Sora moving while KH1 holds him still; without Combo Master one
  swing only):
  - KH1's attack step (`0x1402A4760`) counts each attack in the save's tallies
    (`0x14028A4D0(0x13)`: `+0x848 + id * 2`, capped 9999); event scripts read the tallies
    (`0x14028A4B0`, an event opcode at `0x1401ADF??`). KH2's swings now count too.
  - KH1's player control step `0x1402A4090 (actor, move vector)` (from the think step
    `0x1402A1170`, skipped when actor `+0x444` bit 1 is set) reads the pad and starts
    commands only when `0x1402956E0` says he can act (flag 4, `+0x88` bit 0x20, `+0x444`
    bit 1). Hooked: KH2 gets no input on a frame it didn't run with him able to act.
  - The weapon's grip bone 280: when KH1's own grip (relative to its parent, read from KH1's
    pose while it shows slots 0-7) is turned more than 1 rad from the battle idle's, it is
    used (`Retarget::set_grip`): the Dream Sword is held another way than the Keyblade.
  - `Abilities::one_swing_on_miss` (character-core, ours, set in `abilities = kh1`): without
    Combo Master a swing that hasn't connected doesn't chain.
- `kh1-hook` stage 51 (the user: at the start of some cutscenes Sora slides and snaps back;
  the Dream Shield is held backwards): held still on the ground (KH1's control step skipped
  or him unable to act) for 2 frames, KH1 has him (KH2's run slide or lunge no longer moves
  him while the event sets up), KH2 starts over afterwards. Logged once per weapon: Sora's
  weapon (save `+0x36`) and KH1's grip of it against the Keyblade's rest grip (for the
  shield's fix).
- `kh1-hook` stage 52 (the user: the Dream Shield's front faces Sora): ini `flip_weapons`
  (KH1 item ids, default 0x53, taken to be the Dream Shield: KH1's weapon items run Kingdom
  Key 0x51, Dream Sword, Shield, Rod; unconfirmed, the weapon line in the log gives the id):
  the grip bone's subtree is turned half round about the forearm at the hand
  (`pose::turn_weapon`).
- `kh1-hook` stage 53 (the user's video: with the Dream Shield, KH1's idle and attacks
  showed, a choppy cut out of a run, no attack while walking): the shield has its own
  moveset, whose motions our slot table (P_EX100's signatures) doesn't know, and an unknown
  slot was taken for KH1's own action (KH1 had him). Now only a known slot past 7 is; KH1's
  move hand-back takes an unknown motion as standing when KH1's last request was a slot up to
  7 (`kh1_moving`). Confirmed in the log: the Dream Shield is item 0x53 (`flip_weapons`'s
  default). The live-grip switch of stage 50 is gone (KH1's grip moves with its motions; it
  flapped between the two every few frames); its log line stays.
- `kh1-hook` stage 54 (the user: lock-on on Darkside's hand still sent Sora at his body;
  Wakka's ball can't be hit back; the Dream Shield still faced Sora; KH1's own moves (ladder)
  snapped in; items from chests were Archipelago items):
  - `docs/kh1-lockon.md`: KH1 locks a (handle, lock point) pair: point index at
    `0x142DB3F1C`; `0x1402AE330 (actor, point, out vec4) -> radius` is the point's world
    position (lock points: collision record `+7` count / `+0x1C` list, 0x14 each, mask actor
    `+0x3AC`). KH2's target for the locked actor is now that point.
  - `docs/kh1-reflect.md`: shots like Wakka's ball are free-standing attack objects of the
    pool (`+0x30` bit 0x400); a swing turns them back through the attack-vs-attack meeting
    `0x1402B2890 (entry, entry)` (`0x1402B2A30` both ways: team swapped, Sora the owner, flag
    0x1000000). `combat::shots` / `meet_shot`: KH2's swept swing against them, once a swing,
    with a KH1 row's attack object as the swing side.
  - The Dream Shield (`flip_weapons`): held with KH1's own wrist (the grip's parent bone
    relative to its parent, from KH1's pose each frame it poses him), carrying the fingers and
    the weapon; KH2's arm swings as before. (The half-turn of stage 52 is gone.)
  - KH1's own animation blends in from the last pose we drew over 0.2 s (`KH1_BLEND`).
  - The user's KH1 `Image/en/kh1_first` was a randomizer-patched package (Feb 2025: 26 files,
    Destiny Islands / Dive / Disney Castle `.ard`s, `UK_treasure.ev`, ...): restored from
    `BackupImage/`, the patched pair kept as `*.ap-patched`.
- `kh1-hook` stage 55 (Wakka's ball still not turned back; nothing in the log): the swing now
  meets every live enemy attack of the pool but guards (KH1's pass meets every pair; the
  meeting decides), and each one near Sora is logged once by slot and kind ("enemy attack
  near Sora: ...") to tell what the ball is.
- `kh1-hook` stage 56 (diagnostics for the user's T-pose arms, seen on the ladder and at
  Darkside): logs when KH1's own pass leaves both hands where the bind has them relative to
  the chest (with the pass, slot, the ground-IK pass's float argument (a blend weight, used
  by `0x140392710`'s lerps) and mode), and when the pose we draw does.
- `kh1-hook` stage 57 (the user's ladder video: "T-pose" arms dropping off the ladder): not a
  T-pose but KH2's battle fall (motion 4, arms flung up and out: hands at head height ±55
  apart in `retarget_check`) drawn for KH1's own fall (slot 10) while KH1 had him. KH1's own
  jump / fall / landing (slots 8-13) now show KH1's own animation while KH1 has him (KH2's
  jumps are KH2's as before). The T-pose detector of stage 56 is gone (its chest bone was
  three parents up from the hand, still in the arm: it fired in every idle).
- `kh1-hook` stage 58 (the user: walking into a ladder, the walk froze before the climb):
  `pose` kept drawing KH2's last pose for 0.15 s after KH1 took him (`takeover::POSE`'s
  freshness), then blended from it: a frozen walk for a third of a second. Taking him to KH1
  now drops that pose (`takeover::drop_pose`), and the 0.2 s blend into KH1's own animation is
  eased out (KH1's motion shows at once, our last pose fades).
- `kh1-hook` stage 59 (the user: can't land on boxes; Sora hung over a box in KH2's fall,
  arms out): the map floor point `+0x320` doesn't know objects, and KH2's falls set his height
  against it alone. Falls now go through KH1's mover in its fall state too (as rises did), which
  stops him on objects; a fall stopped short (less than half of it happened) lands him there
  (`Driver::on_top`), and on the ground he stays on top while the ground mover keeps him at
  that height (else the usual floor step-down / walk-off).

### 2026-10-16: Stage 60 (fewer clinks, wall clinks, box tops)
- **Enemy attacks a swing meets** follow KH1's own pass (`0x1402B2610` / `0x1402B1A50`): motion-tied
  attacks (activity 2) only inside their owner's hit window, `0xFFF` latching skipped, word bit
  0x400 and spent 0x80 attacks left out, the pairing table `0x1405299B8` checked both ways. The
  user: endless clinks against the enemies' resting attack shapes.
- **Walls:** each live KH2 swing casts KH1's background ray (`0x140191520`) from last frame's
  attack-shape centres to this frame's; a map face whose normal is mostly level (a wall) repels
  the swing (`register_wall_hit`, KH1's clang). Floors, ceilings and objects don't.
- **Box tops:** KH1's object-top finder `0x1402B3950 (actor, move) -> object` (types 2 / 3
  collision shapes of standable objects) is run in KH1's air state 2 (it skips state 3's class)
  from where a fall started, and each grounded frame a step down; he lands / stands on the top
  and `+0x398` is set (`0x140295C60`). The mover in our states never found them.


### 2026-10-16: Stage 61 (aim only at KH1's target)
- KH2's controller gets only KH1's current target (R1 lock, state 3, or the small cursor's soft
  target, states 1/2/4) in its target list, so swings neither turn toward nor home in on any
  other enemy or lock point; with none, a swing goes where the stick points. Ini `auto_aim = on`
  brings back KH2's own nearest-in-reach pick. The user: the automatic aim was too heavy.

### 2026-10-16: Stage 62 (wall clinks off by default)
- The user changed his mind: swings pass through walls again. The wall ray stays behind ini
  `wall_clink = on` (default off).

### 2026-10-16: Stage 63 (Quick Run LV2 later, both with AP, "New Ability" prompts)
- Quick Run: LV2, learned at level 12 (was 7); Quick Run (2 AP) and Aerial Dodge (3 AP) are in
  Sora's own list now, off until equipped, and their AP joins `bonus_ap`. Saves from earlier
  builds have them moved out of the shared list (one that was on stays on).
- Learning a KH2 ability shows a box in KH1's level-up prompt style ("New Ability", its name,
  its AP), through KH1's prompt boxes `0x14283BD90` / text `0x142DC3068` the way the
  Archipelago KH1 client shows items (gaithern's KH-1FM-AP-LUA, MIT, `show_prompt`); queued
  until Sora's box is free (a level-up's own box goes first).

### 2026-10-16: Stage 64 (walk out of the dodge roll)
- From the roll's fall-check event (frame 34 for Sora) the stick ends the roll and Sora walks /
  runs at once, as out of a landing; before, only a button did, so he stood still until the
  motion ended (the user: too much of a freeze after the roll).

### 2026-10-16: Stage 65 (New Ability box: the name only)
- The "New Ability" box shows the ability's name alone (one line), the user's choice.

### 2026-10-16: Stage 66 (KH2's field camera, Camera Style in KH1's Config menu)
- Research: `docs/kh1-camera.md` (KH1's field camera module, its mode table and state) and
  `docs/kh1-config-menu.md` (the Config menu's window, tables, text lookup, storage).
- **`win/camera.rs`:** KH1's camera mode table (`0x140507AD0`) gets ours for mode 1 (follow)
  and mode 3 (lock-on). Each frame `kh2_camera::Camera` (KH2's free follow / lock-on framing,
  wall pull-in with KH1's camera ray `0x140191640`) writes KH1's internal eye / target, with
  KH1's yaw / pitch / distance kept in step and its own timers zeroed; KH1 still blends to and
  from its other cameras, runs its climbing / ledge cameras (`0x1401E3320`), events and area
  cameras, and builds the View. Right stick: x turns (KH1's left/right option's sign), y zooms;
  R3 resets; Select is KH1's first person; KH1's Auto / Manual option is KH2's Auto. KH1 world
  ↔ camera: y flipped and divided by `camera_scale` (0.9).
- **`win/config.rs`:** a ninth Config row "Camera Style: KH1 / KH2". The window definition's
  row counts go to 9; its update is wrapped (our row's left / right / confirm switch the
  camera and are hidden from KH1's update; the row is drawn with KH1's panel calls; rows 25
  apart during the call so nine fit); the help line's id is swapped for ours on our row; the
  sysmsg lookup `0x1402CE7C0` is replaced (our ids `0x7F000001..4`). The choice is saved in the
  ini (`camera`), not the save.
- Untested in game: the row's layout against the help box, the text codes in sysmsg, the
  camera's turn direction and scale.

### 2026-10-16: Stage 67 (KH2 camera: right stick x the right way round)
- The right stick's x is negated for `kh2_camera` (the user: x was inverted).

### 2026-10-16: Stage 68 (KH2 camera at KH2's own distance)
- `camera_scale` defaults to 1.0 (KH2's distances and heights as they are), the user: a bit
  further back, as in KH2.

### 2026-10-16: Stage 69 (KH2 camera lowered)
- `camera_lower` (default 70): KH2's camera, look point and eye together, that much lower, so
  KH1's smaller Sora sits near the middle of the screen (the user). Same angle as KH2's.

### 2026-10-16: Stage 70 (KH1's camera in cutscenes)
- KH2's camera gives way to KH1's own follow / lock-on camera while an event runs (talks with
  text boxes included), an area script walks Sora, or KH1 holds him still
  (`takeover::kh1_scene`); KH1's carries on from where KH2's was. After a gap KH2's starts
  again from KH1's view (yaw and distance), not from where it left off.

### 2026-10-16: Stage 71 (the Jungle Slider is KH1's)
- KH1 movement state 30 (the Jungle Slider) hands Sora to KH1 for as long as it lasts (the
  user: he walked along the vines instead of sliding); KH2's camera gives way there too.
  `KH1_MODES` in `takeover.rs` lists such states.

### 2026-10-16: Stage 72 (no hanging on the sides of crates and tents)
- A fall the air mover stopped short counts as a landing only when KH1's collision has him on
  ground (`+0x2D0` bit 0 clear); a wall beside him stopped it too, and he hung on the sides of
  crates and the camp's tent, flickering between landing and falling (the user's video).
  Otherwise the fall keeps KH2's own height. On the ground he stays on such a landing while
  KH1's collision keeps him on ground there.

### 2026-10-16: Stage 73 (all of the Jungle Slider is KH1's)
- The slider also runs in KH1 movement state 31 (the user: its jump and attack didn't work, KH2
  had Sora there): every state from 30 up is KH1's (`kh1_mode`).

### 2026-10-16: Stage 74 (catching a ladder on the way down)
- The grab probe's air move presses 4 units along the stick instead of standing still: KH1's
  climb check (`0x1402C5D30`) needs the wall face the move touched (`+0x2DE`), which a
  standing-still move cleared, so a ladder jumped into was caught only at its foot (the user).
  Without a grab the position is put back.

### 2026-10-16: Stage 75 (a roll right after KH1's own move is KH2's)
- While KH1 has Sora back on the ground after one of its own air moves (`HANDOFF`, state 0/1),
  KH1 doesn't see Square, and a Square press ends the handoff at once with KH2 taking it as a
  fresh press (KH2's roll). Before, KH1 started its own roll there (slot 117) and KH2 took over
  mid-way: he slid backwards (the user, jumping off a pole and rolling).

### 2026-10-16: Stage 76 (KH2 camera on entering a room)
- A new KH2 camera starts at KH2's default distance, behind Sora unless KH1's camera was
  looking at him from a usual distance (then its yaw); and one is started when Sora is far from
  the look point (a new room, a warp). Before, it took KH1's left-over pair's distance, and
  KH2's closest distance puts the eye 150 below the look point: on the ground looking up (the
  user, every room entry).

### 2026-10-16: Stage 77 (no clashing with enemies' melee attacks)
- A KH2 swing meets only free-standing enemy shots (`+0x30` bit 0x400: thrown and fired
  things, Wakka's ball among them, whose log entries showed `+0x30 = 0x541`). Enemies' melee
  attack shapes no longer clash with it (the user, again: far too many clinks); the swing hits
  their bodies as in KH2.

### 2026-10-16: Stage 78 (KH2 camera parity)
- KH2's field of view: 69.9° vertical (1.5 rad across 4:3), set through KH1's FOV setter as 76.1
  "PS2 degrees" each frame KH2's camera runs (KH1's 53 is 48° vertical); KH1's default comes
  back when KH1's camera runs.
- Framed as in KH2: `camera_scale` 0.89 (KH1 / KH2 Sora's hips) and `camera_lower` 0.
- Sticks: the pad's raw ones with a 15% dead zone (KH1's processed ones drop the inner 47%);
  KH1's processed right stick when the raw one is idle (mouse).

### 2026-10-16: Stage 79 (New Ability box after the level-up's own)
- The "New Ability" box waits until KH1's prompt box has been free for 4 s: a level-up's own
  box comes a moment after the level changes and replaced ours (Aerial Dodge's was logged as
  shown, the user never saw it).

### 2026-10-16: Stage 80 (New Ability box beside the level-up's)
- KH1's prompt column holds up to 5 boxes per party member (`0xBA0` apart, count 16 bytes before
  the first). For 3 s after Sora learns an ability its box waits for KH1's own (the level-up's)
  and is added as the next box in Sora's column, with its own text buffers; with none by then it
  shows alone once the column is free (the user: show both at once).

### 2026-10-16: Stage 81 (Aerial Dodge after High Jump; grabs as KH1's jump tries them)
- Aerial Dodge is learned at level 13 only once Sora has High Jump (the user); without it, it is
  taken back out of his list, and its AP isn't in `bonus_ap`.
- Air grabs (ladders, vines, poles, ledges) are tried as KH1's jump step does: also near the top
  of the jump (rising under 2 a frame) and with the stick neutral (then no probe move: the
  frame's own air move left the contacts). The user: hit or miss from a jump.

### 2026-10-16: Stage 82 (bosses give KH2 moves)
- `growth::BOSS_GIFTS` (the user's list): Sabor Upper Slash, Clayton Aerial Sweep, Jafar Aerial
  Finish (KH1's Gravity Break), Pot Centipede Guard Break (Stun Impact), Dragon Maleficent
  Explosion (Hurricane Blast), Captain Hook Magnet Burst, Cerberus Aerial Dive. Learned once the
  world's story progress byte (save `+0x1504`, `0x142DEB264`, 12 worlds) reaches the value it has
  after the boss (thresholds as the Archipelago KH1 client reads them), unless Sora already has
  it; off until equipped, with a "New Ability" box. Upper Slash, Aerial Dive and Magnet Burst
  are no longer learned by level (`MenuAbility::level` = `BOSS`); their AP joins `bonus_ap` once
  the boss is beaten.

### 2026-10-16: Stage 83 (Aerial Sweep at level 28)
- `growth::LEVEL_GIFTS`: KH1's Aerial Sweep also at level 28 (the user; KH1's Dream Shield gives
  it at 84), unless Clayton already gave it. Guard Break stays at KH1's level 18 (the user).

### 2026-10-16: Stage 84 (Upper Slash works)
- Upper Slash's PTYA row (12, kind 0, motion 161) was never reachable (kind 0 rows aren't picked
  by `select`). As KH2's help has it: the action button (Square) in a ground combo's combo or
  finisher window starts it (forced row), when equipped. Retaliating Slash also takes the
  action button now (KH2's), besides attack. Menu help texts for both rewritten to say so.

### 2026-10-16: Stage 85 (Aerial Recovery has something to recover from)
- KH1's knock-back hits (kind 4) are KH2 blows now, as its launches (5) were; every 4 was a
  flinch, so Sora was hardly ever knocked into the air (the user: Aerial Recovery never came
  out). Aerial Recovery (jump) also works from an air flinch (hit while airborne).

### 2026-10-16: Stage 86 (KH2's Square moves)
- KH2's Square combo moves are kind-0 PTYA rows, never picked by `select`. The action button in
  a combo or finisher window now tries, in order, Finishing Leap (row 37, ability 95: the
  combo's end), Upper Slash (12, 18) on the ground and Horizontal Slash (38, 99) in the air,
  as KH2's help texts describe them (the user: Horizontal Slash didn't work). Menu help texts
  updated.

### 2026-10-16: Stage 87 (KH2 launches launch)
- Research: `docs/kh1-launch.md` (KH1's hit record, an enemy's launch reaction `0x1402B6390`,
  its arc fields). KH1 has no per-hit launch speed or angle.
- A KH2 hit whose ATKP reaction is a launch (category 4) forces KH1's launch (record kind 5, in
  `scale_record`) and, once KH1's launch reaction has run (hooked) and the enemy did launch
  (motion 0x52 / 0x5C, state 2), sets KH2's apex height and rise time (`+0x3BC` / `+0x3B8`, from
  the ATKP knockback values flown as KH2's blow flies) and horizontal speed (`+0x3C0`). Upper
  Slash goes up (the user). Enemies without a launch motion (big ones, most bosses) don't.

### 2026-10-16: Stage 88 (no attacks mid-launch; KH2 camera through summons)
- A forced combo row (Counterguard, Retaliating Slash, the Square moves) that can't be used is no
  attack: it fell back to the ordinary combo, so Sora attacked while knocked into the air (the
  user) and Square mid-combo could start a plain swing.
- KH1 holding Sora still counts as a scene for the camera only out of battle (the user: KH2's
  camera should stay through a summon's attack, Simba's charge).
- Launch logs: each KH2 launch asked for, and KH1 launch reactions that weren't ours.

### 2026-10-16: Stage 89 (a tapped Quick Run goes less far)
- Letting go of the action button before Quick Run's time is up stops it with a stronger brake,
  the earlier the stronger (0.7 per tick for a tap, KH2's stop brake when held to the end): at
  LV2 a tap went ~740 units against ~950 held, as KH2's end slide is most of it (the user).

### 2026-10-16: Stage 90 (KH2 camera through Simba's charge)
- Only the Jungle Slider's movement states (30, 31) give the camera to KH1; a summon's (38
  Simba's charge, 40 the call) keep KH2's (the user). KH1 still has Sora in all states 30+.

### 2026-10-16: Stage 91 (Quick Run grows with Sora; Aerial Recovery as the exe)
- The tap brake (stage 89) is gone (the user). Quick Run grows with Sora instead
  (`growth::QUICK_RUN_LEVELS`): LV1 at 12, LV2 at 20, LV3 at 32, MAX at 45. Its menu name is a
  buffer rewritten in place ("Quick Run LV2" ... "Quick Run MAX"), and each growth shows an
  "Ability Up" box.
- Aerial Recovery per `docs/aerial-recovery.md` (exe `0x1403D6030`): launches only (not air
  flinches), from the launch's 16th frame, jump: motion 40, hop 100, 10/frame along the stick
  (straight up with it neutral), invulnerable to frame 16, control back at frame 24, no Aerial
  Dodge after. Retaliating Slash: Square only, same frame.

### 2026-10-16: Stage 92 (Dodge Roll shows its level)
- KH1's Dodge Roll (22) gets a name buffer like Quick Run's ("Dodge Roll LV1" .. "MAX", its
  levels as before: 19 / 37 / 60), written into our ability table's entry, and an "Ability Up"
  box each time it grows (the user).

### 2026-10-16: Stage 94 (High Jump and Aerial Dodge grow with Sora)
- High Jump (KH1's 1, shared) and Aerial Dodge (our 67) level up like Quick Run and Dodge Roll
  (the user): High Jump LV2 / LV3 / MAX at 24 / 38 / 52, Aerial Dodge at 25 / 36 / 50
  (`HIGH_JUMP_LEVELS`, `AERIAL_DODGE_LEVELS`), LV1 when Sora gets them. Each gets a name buffer
  ("High Jump LV2", entry 1's handle like Dodge Roll's; Aerial Dodge's in the TEXT init like
  Quick Run's) and an "Ability Up" box when it grows; learning Aerial Dodge shows its level.

### 2026-10-16: Stage 95 (no leftover launches)
- The user's log: Upper Slash's launch reaches Shadows (up 265 in 26 frames), but launches
  asked on Cerberus (which KH1 never launches) stayed queued under KH1's reused hit record and
  one landed on a Shadow later. A KH2 launch now waits only 250 ms (`LAUNCH_FRESH_MS`) for
  KH1's launch reaction.

### 2026-10-16: Stage 96 (ground-to-air rows)
- The user: Aerial Sweep and Aerial Dive (ground-to-air) were hard to trigger. KH2's combo
  selector (`0x140401BB0`) measures facing from the direction to the primary target, not the
  player's facing; kind-1 rows have a facing window of 0.99996..1, so with Sora's own facing
  they (Aerial Sweep 33, Aerial Dive 48, the jump at an airborne foe 0, Slide Dash, Flash
  Step, the long approach 153, Aerial Spiral) almost never passed. `Controller::try_attack`
  now aims at the primary target for the selection. The height window's sign was checked too
  (`0x140401AD0`: target − player, y-down; the doc had it reversed, the code was right).
- Aerial Dive's help: from the ground, at a far airborne foe.

### 2026-10-16: Stage 97 (four shortcuts)
- The user: four shortcut slots like KH2, KH1's shortcut menu extended so it fits in.
  `win/shortcuts.rs`, `docs/kh1-shortcuts.md`. The fourth is L1 + the cancel button (Circle;
  Cross with the button swap), kept at save `+0x847` (spell + 1; old saves start empty), set
  in Customize (a fourth row, its label KH1's third with the cancel icon), filled with a newly
  learned spell when KH1's three are full. In battle the shortcut menu moves up a row, its
  panel is stretched by a row (cut at the middle, a one-texel band between), and the fourth
  row gets the cancel icon. `shortcuts = 3` in the ini turns it off.
- The text lookup (`config.rs`) is in even with `config_row = off`; an id past KH1's table reads
  as empty.
- The previous run's log is kept as `kh1-kh2.prev.log`.

### 2026-10-16: Stage 98 (the shortcut menu looks like KH1's)
- The user's screenshots: the battle panel cut in the middle looked wrong and the title
  ("SHORTCUT", the second panel: 8 px tall) moved into the rows (its quad is in GS
  coordinates, not the menu's, so the "lower rows" test always passed). The panel quad (header
  and three rows, 86 px, UV 1:1) is now drawn whole and its last 22 px drawn again a row lower;
  the title stays. In Customize the fourth label showed garbage: KH1's label 0xD0 is
  `0D 16 00 ...` (an x offset whose operand has a 0 byte), so it was cut short; the label is
  now read code by code (`0x1402CFEA0`'s operand sizes) and the confirm icon (`0F 41`) becomes
  cancel's (`0F 42`).

### 2026-10-16: Stage 99 (the gradient over four rows)
- The user: the shortcut panel's gradient should go all the way down. The panel is drawn as its
  header and four 22 px rows cut from KH1's three; row k shows texture row 2k/3, the two
  nearest rows blended (the second drawn over with vertex alpha), so the shading spreads over
  four rows and the shapes stay KH1's. The Customize label came out `0d 16 00 0f 42` (right).

### 2026-10-16: Stage 100 (lock-on camera steady)
- The user's video (KH2 camera, locked on in the Cave of Wonders): the camera jumped about.
  The "Sora is far from where it was looking" restart compared the look point with his feet;
  locked on, the look point is 30% of the way to the target, so a far target restarted the
  camera (from KH1's eye) again and again. It compares the eye now (1200). A lock-on target
  with no point this frame no longer hands the frame to KH1's lock-on camera: ours follows.
  Restarts and hand-overs are logged with their reason.

### 2026-10-16: Stage 101 (targets as KH2's target shapes)
- The user: Aerial Dive still barely came out. KH2's combo table measures a target by its
  target shape (`0x1403C8EE0`: the shape's point raised by its position height, and its own
  size), and Aerial Sweep / Aerial Dive / the jump at an airborne foe (flag 0xA: the window's
  low end shrinks by the target's height) want that point at least 110 + its size above Sora.
  We gave targets their feet and whole height (a flyer needed its feet ~200 up). Every target
  is now KH1's lock point 0 (its centre, the lock radius as its size), the locked one too (it
  was the sphere's bottom and twice the radius); a target without lock points keeps its feet
  and height. Magnet Burst's pull still aims at the body's middle (`Body::tall`).

### 2026-10-16: Stage 102 (the magic carpet)
- The user's video: the magic carpet out of the Cave of Wonders (after Genie Jafar) didn't
  move. The log: KH1's control step for Sora came and went every few frames ("KH1 holds Sora
  still" / "lets Sora act again" over and over), and KH2 took him on the frames between, so
  KH1's own physics (which the ride needs) didn't run. KH1's control coming and going 6 times
  in 3 s is now taken for a ride: KH1 has him while it lasts and 3 s after. The control step
  is read once per frame at the start of `takeover` (`FROZEN_NOW`), before any early return.
  The hold / act lines now carry his state, slot, flags, motion and the object he stands on.

### 2026-10-16: Stage 103 (Aerial Finish holds)
- The user: enemies were pushed out of the air finisher. Aerial Finish's spinning hits
  (ATKP 915 / 916 / 917: reaction 9, up 10 / 5 / 12, sideways -1 / -1 / -10) are blows that
  hold the foe up beside Sora; landed with KH1's air finisher row they sent it ~250 away on the
  first hit (the log: 915 at z -1836, 916 at -2089) and the rest missed. A blow takes KH1's
  finisher row (and a KH2 launch) only when it throws: sideways >= 10 or up >= 30 (or the
  default blow); the others land with the plain row. The last hit (927: 3 / 45) still throws.

### 2026-10-16: Stage 104 (invulnerable frames)
- The user: Aerial Recovery, Aerial Dodge and Quick Run should have invulnerable frames.
  Aerial Recovery already had KH2's (to frame 16). Aerial Dodge (none in KH2) gets 10 / 12 /
  14 / 16 by level (`AIR_DODGE_STAR` + 2 per level). Quick Run keeps KH2's fmab star times
  (0 / 8 / 12 / 16 by level) with at least 6 (`QUICK_RUN_STAR_MIN`, for LV1).

### 2026-10-16: Stage 105 (Riku's Aerial Dodge, later growth, holding hits stay put)
- The user: Aerial Dodge for beating Riku the first time (Hollow Bastion progress 0x32, its
  reward's threshold in gaithern's KH-1FM-AP-LUA, MIT; 0x28 during the fight), no longer a
  level or High Jump's (the High Jump gate and its take-back are gone; the user's Archipelago
  save has no High Jump). Its New Ability box shows its level. High Jump grows at 30 / 44 / 58,
  Aerial Dodge at 40 / 52 / 64.
- Still sliding out of Aerial Finish: KH1's flinch on an airborne foe (kind 3, `0x1402B69E0`)
  knocks it along the record's direction each spin. A holding blow's record direction is now
  zeroed (in `scale_record` and again after the contact), so it flinches in place.

### 2026-10-16: Stage 106 (no Sonic Blade in the air)
- The user: Sora could use Sonic Blade mid-jump. KH1's command check (`0x1402912F0`) allows
  only magic (kind 2), kind 99 and kind 0x47 in the air, judging the air by his flags
  (`0x1402A0D30`: `+0` bit 4, `+0x88` bit 0x20, `+0x444` bit 1), which stay the ground's while
  KH2 has him. `combat::command` now refuses any other command while KH2 has him in the air,
  with KH1's "can't" sound (`0x140177F60 (4)`). Magic stays allowed (jump casts).

### 2026-10-16: Stage 107 (Aerial Dodge taken back until Riku)
- The user's save still had Aerial Dodge (from its old level). It is taken out of Sora's list
  while Riku (Hollow Bastion, first time) isn't beaten, and given again (with its box) once he
  is.

### 2026-10-16: Stage 108 (KH2's sounds for every ability move)
- The user: the exact KH2 sounds for the KH2 moves, as Explosion's and Magnet Burst's (no
  visuals). `kh2sound::DEFAULT_MOTIONS` is now every ability motion of Sora's PTYA (161..172,
  191..197: Upper Slash, Slapshot, Dodge Slash, Slide Dash, Guard Break, Explosion, Finishing
  Leap, Flash Step, Vicinity Break, Counterguard, Retaliating Slash, Aerial Sweep, Aerial
  Spiral, Horizontal Slash, Aerial Finish, Aerial Dive, Magnet Burst); the plain combo keeps
  KH1's. `kh2_effects` keeps its own two (`kh2fx::DEFAULT_MOTIONS`). Any `kh2_sounds` word
  but `all` / `off` (the old `magnet_explosion` too) means the new default.

### 2026-10-16: Stage 109 (no Sonic Blade in the air, again)
- The user: Sonic Blade still came out mid-jump; the log had it as a ground command (command
  0x4, not "in the air"). `kh2_airborne` read the driver with `try_lock`, and KH1's command
  step can run while it is held (a failed lock read as "on the ground"): the takeover now keeps
  `KH2_AIR` each frame. The refusal also runs while KH1 has him in its fall state 2 (after one
  of our air casts, whose state we set without KH1's airborne flags), before the hand-off
  test, and every command logs whether KH2 had him in the air.

### 2026-10-16: Stage 110 (the ride test only while KH2 has him)
- The user: Sora stuck in KH1's animations. Stage 102's ride test counted KH1's control coming
  and going on every frame, KH1's own included (a room entry's script holds, then KH1's own
  frames, which flicker too), and each count kept the ride going: KH1 had him for good. It now
  counts only on frames KH2 has him, needs 10 changes in 3 s, and gives KH1 him for 5 s
  without extending.

### 2026-10-16: Stage 111 (the magic carpet, for certain)
- The user can't easily get back to the carpet and it softlocks if broken. `0x1402957F0` (the
  player actor's setup) puts Sora in movement state 29 (0x1D) in Agrabah (world 8) room 0x14
  while its event index is under 0xD: the carpet out of the Cave of Wonders. KH1's own modes
  were 30 and up; now 29, 24 and 25 too (24 is Atlantica's water, set when `0x14298D3A0`, world
  9 or End of the World room 0x15, `0x140288920`; 24 / 25 are the states where jump means up,
  `0x140291660`'s mask 0x203000100). KH1's camera for 24 / 25 / 29 / 30 / 31. (Stage 110's
  flicker test stays as a fallback.)

### 2026-10-16: Stage 112 (KH2-named KH1 abilities earlier)
- The user: Vicinity Break at 69 (Dream Shield) is absurd, Slide Dash at about 30. `LEVEL_GIFTS`
  now also gives Counterguard (Counterattack) at 22, Slide Dash (Sliding Dash) at 30, Vicinity
  Break (Ripple Drive) at 36, Flash Step (Vortex) at 42 and Finishing Leap (Blitz) at 48, with
  Aerial Sweep at 28 as before (off, with a New Ability box, if Sora doesn't have it yet).

### 2026-10-16: Stage 113 (Aerial Finish holds its foe)
- The user: foes still slipped out of Aerial Finish. The log: whole spins land sometimes, but
  often the foe is gone after 915 / 916 (falling while Sora spins up). KH2's reaction-9 blows
  lift and pull the foe toward Sora; KH1's flinch lets it fall. `Strikes::hold`: a foe a
  holding blow hit is drawn, for 24 frames after each such hit, toward a point no more than 90
  from Sora at his height + 30, through KH1's per-frame push (`+0x120`, at most 30 a frame).

### 2026-10-16: Stage 114 (Gummi routes row, chests in battle)
- The user: a Config option to skip the Gummi routes or fly them; chests in battle.
- `config.rs` now has two rows of ours (8 Camera Style, 9 Gummi Routes: Vanilla / Skip), ten
  rows 22 apart while the list runs, help ids swapped in at the table's slots 8 / 9;
  `settings::save(key, value)` keeps either in the ini (`gummi = vanilla / skip`).
- `gummi.rs` ports KH-1FM-AP-LUA's Instant Gummi (denhonator, Gicu; MIT; credited in NOTICE /
  CREDITS) to the pad read each frame: the chosen destination becomes the flight's current
  one (Epic addresses: selection 0x507D7C, destination 0x508280, 0x507C90, 0x268A26C, warps
  0x50F9D0).
- `chests.rs`: the battle check before the chest prompt (`0x1402B3904`: `test [0x142867CD8],
  1; jz`) becomes `jnc`, as the AP interaction script (KSX, Gicu) does. `chests_in_battle`.

### 2026-10-16: Stage 115 (big targets by their parts)
- The user: Sora couldn't hit the glowing blobs on Oogie's Manor. Likely parts of the manor
  actor, whose root is far from Sora: `combat::bodies` dropped any actor whose root was more
  than 1500 away. It now keeps an actor when any of its parts is within 1500 (KH1's own hit
  pass tests every part), and logs big targets (more than 8 parts: parts hittable, root
  distance) for the next look.

### 2026-10-16: Stage 116 (big targets aren't pulled)
- The user: Aerial Finish (and likely Magnet) on Oogie's Manor's blobs pulled the whole house
  into the floor: our pulls push the whole actor (`+0x120`). A body is `fixed` (more than 8
  parts, root beyond 1500, taller than 600 or radius over 300) and the Magnet pull and the
  Aerial Finish hold skip it. Stage 115 and 116 shipped together.

### 2026-10-16: Stage 117 (dev tool: insta-kill)
- The user asked for a dev tool to toggle insta-killing enemies. New `win/devtools.rs`:
  `insta_kill = on/off` in the ini (default off); F8 (monitor thread) or L1 + R1 + Select (pad
  hook) toggles it and saves it with `settings::save`; menu sound 2 (on) / 3 (off), played on
  the game's thread from the pad hook. With it on, `probe::hit` (every hit record from an
  attack Sora owns: KH2 swings after `scale_record`, KH1 swings, magic) gets damage = the
  foe's HP (stats `+0x3C`), at least 9999.

### 2026-10-16: Stage 118 (Neverland flight)
- The user: Sora couldn't fly in Neverland. KH1's flight is movement state 8, not 25 (25 is
  Mermaid Kick's dash, from swimming state 24 with KH1 ability 2). State table at
  `0x142DB4240` (0x40 per state), filled by `0x1402AF9F0(state, flags, enter, exit, ...,
  move +0x20, next +0x28, ..., +0x38)`; the setter is `0x1402AF7D0(actor, state)`. The fall
  state (2) starts flight when the world flags `0x142867CD8` have bit 0x20 (Neverland) and
  `+0x3D4` (height above the floor) is over 100 (`0x1402AFD30`: motion 0x75, state 8,
  `+0x3C4 *= 0.3`), or by a per-room height table (`0x1402AFC10`, `0x140529970`: world 0xD
  rooms 8 / 9, world 0x10 rooms 0x1A / 0x21 / 0x22). KH2 has the jumps, so KH1's fall state
  never ran: `takeover` now does that check while KH2's jump comes down and calls KH1's flight
  start; state 8 is KH1's own mode (and camera) until he lands.

### 2026-10-16: Stage 119 (KH2 camera in flight)
- The user: flying snapped to KH1's camera with KH2's chosen. Flight (state 8) runs in KH1's
  mode-1 follow camera, which is ours, so state 8 left `kh1_camera_mode` (still KH1's own
  movement mode for `takeover`).

### 2026-10-16: Stage 120 (KH2 aerial attacks while flying)
- The user asked for KH2's aerial attacks in flight. `combat::exec`: the Attack command while
  Sora's KH1 state is 8 sets `takeover::FLY_ATTACK` instead of KH1's flying attack. `takeover`
  then puts him in state 0 (flight's exit step runs), KH2 starts fresh in the air
  (`d.grounded = false`) with that press (`FLY_PRESS`, after the fresh start's dropped
  presses; the "KH1's own action" slot check skipped that frame), and `FLEW` lets the flight
  start again once the combo is over (KH2 state Air / Jump, coming down) at any height above
  20, not only above KH1's 100; cleared on landing.

### 2026-10-16: Stage 121 (no sinking in a flight combo)
- The user: Sora should stay in flight through the combo. While `FLEW` and KH2 is attacking
  (Attack / Magic / Aerial Dodge, in the air), KH2's downward displacement is dropped (rises
  kept); the flight takes him again when the combo ends.

### 2026-10-16: Stage 122 (ladders caught on the way up)
- The user: jumping into a ladder didn't put him on it at once. KH1's jump step `0x1402ADA90`
  tries its grabs every frame, rising too; it skips them only with flag 0x10000 for 15 frames
  (`0x1404308B0`) after jumping off something, or while `+0x3B0 <= +0x3B8 + 6` when `+0x3B8`
  is set. Our "rising under 2 a frame" limit (stage 81) is gone.

### 2026-10-16: Stage 123 (flight combos: sparkle, homing, no landing)
- The user: keep the flight sparkle through flying attacks, home in on the enemy (down too),
  and don't end the flight when a move touches the ground unless Square (descend) is held.
- Sparkle: Sora's step `0x140295E50` keeps its effect at `+0xB0` (`0x1402ABA90(0x142D67400,
  0x14, sora, 0x8003)`) while state 8 and ends it (`byte = 1` on its `+0x10`) otherwise.
  `takeover::sparkle_stub` replaces `cmp [rdi+0x70],8; jnz; mov eax,[rdi]` at `0x14029602E`
  with a call to a stub that also answers "flying" while its flag byte is set (a flight
  combo: `FLEW` and airborne). (The 0x42D4 flight sound and the low-flight dust `0x1402BFEB0`
  are flight's move step's own.)
- Homing: KH2's approach is level; in a flight combo's attack the feet go toward the aimed
  target's point less 60 (lock, else the nearest within 600) at 0.15 of the gap a frame, at
  most 14.
- No landing: in a flight combo the fall stops 15 above the floor unless Square is held; the
  flight takes him again from 5 up.

### 2026-10-16: Stage 124 (no Superglide; Glide grows) and 125 (Aerial Dodge from Genie Jafar)
- The user: remove Superglide; Glide levels with Sora, LV2 at 40 up to LV3; Chernabog gives
  Glide MAX instead of Superglide. `growth::GLIDE_LEVELS = [40, 52]`, `glide_level(level,
  progress)` (4 once `CHERNABOG`, End of the World index 11 at 0x33, the AP client's
  Superglide check); `win/menu.rs` turns Superglide in the shared list into Glide (or drops it
  beside one), names Glide with its level and Superglide's table entry "Glide MAX" (KH1's own
  Chernabog reward box), and an "Ability Up" box for LV2 / LV3.
- Also (the user): Aerial Dodge is Genie Jafar's gift now (Agrabah index 4 at 0x53: 0x46 during
  Jafar, 0x53 after Genie Jafar's cutscene, KH-1FM-AP-LUA's warp fixes), not Riku's, and grows
  from 30: `AERIAL_DODGE_LEVELS = [30, 42, 54]`.

### 2026-10-16: Stage 126 (every enemy counts for the combo table)
- The user: Magnet Burst hardly came out. KH2's Magnet Burst is an air finisher: row 49 (kind
  2: any target within 650 whose direction is more than ~22 degrees off the aim, facing window
  -1..0.924) and row 50 (kind 1: the primary at 300..650). With `auto_aim` off the hook cut
  `Env::targets` down to the lock, so kind-2 rows (Magnet Burst's, Explosion's 26, Vicinity
  Break's 51, 34 / 59 / 43) only ever saw the primary, dead ahead. Now all targets go in and
  `Character::set_auto_target(false)` keeps the aim on the lock only (no lock: no primary).
  Flight-combo vertical homing follows the lock (or the nearest with `auto_aim`). Its help
  text says "Air combo finisher".

### 2026-10-16: Stage 127 (crash after Riku 1: GPU device lost; notes and a safer ring)
- The user: the game crashed after beating Riku 1. The log: `CRASH 0xc0000005 at
  exe+10ab69 (reading 0x0)` in `0x14010A990`, KH1's frame-fence wait reading a finished
  list's GPU timestamps: `ID3D12Resource::Map` (vtable +0x40) on the list's readback buffer
  gave null, which it does once the device is removed (hung / reset). Cause not in the log.
- The crash note now adds KH1's device's `GetDeviceRemovedReason` (vtable 37), our KH2 effect
  draw count and how long ago the last was (`fx12::crash_note`).
- `fx12` vertex ring: a write point going round the whole 24 MB ring (256-byte aligned)
  instead of one 6 MB slot per draw call, which came round again within KH1's frames in
  flight when a frame had several effect draws.

### 2026-10-16: Stage 128 (no KH1 combo during KH1's own moves)
- The user saw KH1's Ripple Drive graphic twice as Riku 1 fell (right before the crash). The
  log's last hits were KH1's own attack objects (`probe::hit` with no KH2 power): Attack
  pressed while KH1 still had Sora after a spell (`hand_to_kh1`, "command 0x2") ran KH1's own
  combo (its finisher: Ripple Drive, equipped). `combat::exec` now takes that press for KH2
  (`takeover::ATTACK_AFTER`): the move ends as soon as he stands free (state 0 / 1, 10
  frames in, as KH1 allowed the cancel) and KH2's first frame gets the press
  (`FLY_PRESS_GROUND`, past the fresh start's dropped presses). Cleared on each new hand-off.

### 2026-10-16: Stage 129 (KH1 moves end on the stick; no spare early gifts)
- The user: after a Hollow Bastion crystal (command 0x5), moving within a second kept KH1's
  animations: the hand-off ended only once KH1's slot was a plain idle / walk / run (0..7), and
  KH1 walked him in other slots (75 / 77). Now it also ends when KH1's own "can act" test
  (`0x1402956E0`: no action flag 4, no reaction `+0x88 & 0x20`, `+0x444 & 1` clear) passes
  with the left stick (pad bytes 2 / 3) past 0.3, state 0 / 1, 10 frames in.
- The user: a second Slide Dash from Maleficent. `growth::early_kh1()` (the KH1 ids of
  `LEVEL_GIFTS` and `BOSS_GIFTS`) keeps one copy each in Sora's list (`keep_one`, the
  equipped one); KH1's own later copy is taken out (logged, and in `kh1-kh2-removed.txt`).

### 2026-10-16: Stage 130 (no "learned" line for spares; fewer clinks; nothing big or fixed moved)
- The user: remove the learned box for duplicates. KH1's event command `0x1401B6990(event)`
  adds a reward's line to a message window (reward id at `+0x198 + sp * 4`, `sp` at `+0x194`;
  record `0x14028D810(id)`: u16, kind = low 4 bits (1 ability), member bits 4..7, ability the
  high byte; it pops 2, returns 2); abilities are added by `0x1401AF120` through
  `0x140282CA0(id, member)` (appends `id | 0x80`). Hooked (`menu::reward_line`): Sora's ability
  that `Kh1Sora::spare_gift` (the mod's level / boss gift is due and he has it) is popped with
  no line. Untested: a window whose only line it was may show empty.
- The user: Darkballs and Dragon Maleficent clinked way too much. Their guards (priority 100,
  objects 330 / 332, flags with bit 20 "spent", which only skips KH1's wall test in
  `0x1402B1670`) repelled swings whose wide KH2 shapes touched a guard beside the part they hit.
  A guard repels now only when it is met earlier along the frame's sweep than any body part.
- The user: Magnet Burst and Aerial Finish shouldn't affect Dragon Maleficent, big enemies, or
  anything immovable. `fixed` also covers actors that aren't KH1 kind 3 (attackable objects);
  a fixed body gets no KH2 launch (`PENDING`, which forced KH1's launch reaction), no Aerial
  Finish hold and no Magnet catch: KH1's own reaction stays.

### 2026-10-16: Stage 131 (KH1's clash table for guards; big stays big)
- The user: Dragon Maleficent was still pulled into a wall (Aerial Finish, Magnet Burst): her
  size was judged each frame from her pose (615 tall standing, under 600 head down). A target
  found big (more than 8 parts, taller than 600, radius over 300) stays big (by actor and
  object id).
- The user: high aerial combos on her clinked. Her big guard (radius 150, flags `000d6a09`) is
  clash type 6, which meets nothing in KH1's clash table (`0x1405299B8`: `1F 09 09 3F 29 18 00
  00`). Guards now repel only when the table lets their type meet Sora's swing type (his KH1
  row data's `+0x38 & 7`); logged ("guards near").

### 2026-10-16: Stage 132 (guards meet a thin blade)
- The user's Dragon Maleficent video: half the air combos' first swings were repelled by her
  guard (radius 180, flags `00116a09`, clash type 0, motion-bound), the other swings hit her
  head / neck. KH1's swing is one sphere of its data's `+0x5C` radius on the blade; KH2's hit
  shapes are wide and clipped the guard's edge on the way to the head. Guards are now tested
  against the sweep with each shape's radius cut to that KH1 radius (8..60).

### 2026-10-16: Stage 133 (no KH1 glide after an air spell; fewer guard repels)
- The user's video: after a spell in the air the hand-off ended with KH1 still in its air
  state (1, then 2); its fall state's step (`0x1402ADBD0` -> `0x1402B0530` -> `0x1402B0BA0`,
  with Glide's flag `0x142D5F610 & 0x80` and the jump button held for KH2's glide) put him in
  KH1's glide (state 0x15, motion 0x71). A hand-off now ends by setting KH1's state to 0
  (`0x1402AF7D0`, its exit steps run) and starts KH2 in the air when he is (`AIR_START`); KH1's
  glide state under KH2 is turned back to 0 at once (logged).
- Clinks: Sora's swing data (bone 0x30) has no radius / priority of its own (the Keyblade's
  are added by `0x1402B10E0`), so "blade radius 8" was only the clamp. KH1 itself clashes a
  type-0 priority-100 guard with any swing entering it. Now, with a part touched in the same
  frame, a guard repels only when that part's centre is inside the guard (the blade must pass
  through it); repel notes log the guard's motion tie, the owner's motion and the nearest part.

### 2026-10-16: Stage 134 (Heartless form is KH1's; damage fractions carried; damage_scale)
- The user: Heartless Sora (Hollow Bastion) had no jump animation, and should have no KH2 at
  all: KH2 drove him while his body was another model, left in KH1's idle. `pose::sora_body_seen`
  (Sora's own 293-bone model posed within 500 ms, from the hands' timestamp) gates `takeover`:
  otherwise KH1 has him ("Sora is in another body").
- The user: more damage, Explosion should do a lot more. Its burst is ~20 hits at power 25
  plus a 150 blast; each hit is KH1's damage x power / 100, which was floored (6 x 25% = 1.5 ->
  1, a third of the burst). The fraction is now carried per foe (`CARRY`, 1/10000ths, the foe
  from the part `land` hits: `TARGET`), and the ini's `damage_scale` (default 1.0, 0.1..10)
  multiplies Sora's KH2 hits.

### 2026-10-16: Stage 135 (Magic-based KH2 hits grow with MP)
- The user: Explosion and Magnet Burst should do more; they scale with magic. KH2's ATKP stat
  source byte says so: Explosion 911 / 912, Magnet Burst 1900 / 1901 / 1903 are 1 (Magic).
  KH1 has no Magic stat: such hits (`MAGIC` while landing) get x (1 + 0.1 x Sora's max MP, save
  member `+8`) x the ini's `magic_scale` (default 1.0) on top of `damage_scale`.
- The user hadn't seen Vicinity Break: it did come out twice in the log (motion 170, attack
  1869, both hits landed). Its row (51) wants a foe behind Sora within 300 (facing window
  -1..-0.17), which before stage 126 never had any foe but the lock. Its KH2 visual stays off
  (the user asked for only Explosion's and Magnet Burst's).

### 2026-10-16: Stage 136 (Vicinity Break's KH2 visuals)
- The user asked for them: `kh2fx::DEFAULT_MOTIONS` is Explosion (166), Vicinity Break (170)
  and Magnet Burst (197); the ini's `magnet_explosion` value now means those three.

### 2026-10-16: Stage 137 (Flash Step for Sabor)
- The user: Flash Step (KH1's Vortex) unlocks after Sabor. Out of `LEVEL_GIFTS` (it was 42),
  into `BOSS_GIFTS` (Deep Jungle index 1 at 0x42, beside Upper Slash). Still one of
  `early_kh1()`: KH1's own later Vortex is a spare.

### 2026-10-16: Stage 138 (Glide on Square)
- The user: Glide on Square, as KH1's Superglide. The controller's `GlideButton::Action` (KH2's
  own option): Square in the air glides; ini `glide = square` (default) or `jump`. Out of a
  flight (`FLEW`) it stays on jump: Square is flight's descend there.

### 2026-10-17: Stage 139 (Gummi skip and insta-kill removed)
- The user, to keep the mod modular: the Gummi route skip is gone (`win/gummi.rs`, the
  `gummi` ini key, the Config menu's Gummi Routes row: back to one row of ours, Camera Style,
  rows 25 apart) and so is the insta-kill dev tool (`win/devtools.rs`, `insta_kill`, F8 /
  L1 + R1 + Select). NOTICE / CREDITS no longer list Instant Gummi.

### 2026-10-17: Stage 140 (release package)
- The user is releasing the mod. Fresh installs default to `abilities = kh1` (was `test`);
  `abilities = remove` (= test mode) takes the KH2 abilities back out of a save before
  uninstalling. The DLL checks the game build first (`supported_exe`: the first bytes of
  KH1's physics step `0x298230`, control step `0x2A4090` and reward line `0x1B6990`, the
  Epic Games Store release) and stays off otherwise (Steam's addresses differ). `fx_test`
  left out of the default ini. The mod.yml describes the mod for players; the release zip is
  mod.yml + kh1-kh2.dll + README.txt (install steps) + licences, NOTICE, CREDITS.


### 2026-10-17: Stage 141 (no chests in battle)
- The user: remove the chests-in-battle patch. `win/chests.rs` and `chests_in_battle` are gone;
  chests stay closed in battle, as in KH1. NOTICE no longer carries KH-1FM-AP-LUA's licence
  (no code from it is left); CREDITS keeps a facts-only line for its story-progress values.
  The prompt-box layout (`menu::prompt`) is checked against KH1's own box updater
  `0x140270DA0`: state at `+0`, timer `+0xC`, line count `+0x18`, lines `+0x20` / `+0x28`,
  title `+0x30`, fade `+0xB80`, colours `+0xB88` / `+0xB90`, 0xBA0 a box, counts per member
  at `0x14283BD80`, members `0x3A20` apart.

### 2026-10-17: Stage 142 (Steam release)
- The user now has the Steam release (`D:\Steam\steamapps\common\KINGDOM HEARTS -HD 1.5+2.5
  ReMIX-`); players reported the mod doing nothing there (it switched itself off). Same game,
  linked the same day (Epic stamp 1721648200, Steam 1721645006), but each build links its store's
  SDK, so code and globals shift by region (e.g. battle flags `0x2867CD8` -> `0x2867364`, save
  `0x2DE9D60` -> `0x2DE9360`).
- `win/addr.rs`: every game address is an `Rva` (the Epic RVA); `base + rva` gives the running
  build's address (`impl Add<Rva> for usize`), from `steam_map.in` (Epic -> Steam, 174 pairs) on
  Steam. `addr::detect` picks the build by the PE link stamp and checks three prologues; any
  other build stays off as before. `tests/steam_map.rs`: every `Rva(0x..)` in `src/win` has a
  Steam address.
- How the table was made (locally, from the user's two executables; nothing of them is in the
  repo): functions paired through `.pdata` and a diff of their instruction streams with
  relative targets set aside (14,827 of 15,824 identical); globals from the RIP-relative
  operands of paired functions. Each code address was checked over 64 bytes, instruction by
  instruction, with each reference mapping consistently; menu tables with no direct reference
  (`0x5470E0`, `0x547130`, `0x547140`) by their contents (-0xCF0, as `0x5470C0`). Patched bytes
  (shortcut rows, Customize, sparkle test, combo table) are identical in Steam's build.
- Steam's English files are in `Image\dt` (the KH1 / KH2 loaders already try en, dt, jp).
- README / mod.yml: Epic or Steam; README.txt kept in `crates/kh1-hook/openkh/`.

### 2026-10-17: Stage 143 (Steam: first test)
- The user's first Steam run (new game): the build was recognised and KH2's control ran, but
  two hooks were refused: menu text lookup (`0x2CE7C0`) and shortcut get (`0x282F20`, and so
  set `0x283640`) begin `mov rax, [rip + disp]`, whose offset differs per build. Their
  expected bytes are now made for the running build (`addr::mov_rax_rip`: text table
  `0x2E1B650`, save pointer `0x28695A0`); the trampolines are never called (both replaced
  outright). Every other hook's prologue checked against both executables: identical.
- Dive to the Heart's "Press to attack" window stayed up although KH2's swings add to the
  attack tally (`0x14028A4D0(0x13)`, identical in Steam's build; read by event opcode 474,
  `0x1401ADFA0`). Diagnostics: event opcodes per second when they change (dispatcher
  `0x1401B13F0`, table `0x1404FB550`, 764 opcodes; `probe::event_trace`) and camera jerks
  (`camera::jerk_note`: the user found the camera jittery there).

### 2026-10-17: Stage 144-145 (the attack tutorial; camera timing)
- The trace (`probe::event_trace`): at Dive to the Heart's "Press to attack" window the script
  polls opcode 322 (`0x1401AB6E0`: pushes the script actor's `+0x164`) every frame, with Sora as
  the actor. `+0x164` is the motion request applied (`0x14029DAB0`: low 16 bits of the first
  queued request): 0 idle, 1 walk, 2 run under KH2; KH1's swing starts the row's motion
  (`0x1402A17F0` -> `0x14029D7F0(actor, motion | 0x1000000)`, 200 for the first ground row),
  which the window waits for. Stage 50's tally (`0x14028A4D0(0x13)`) was not what it reads.
- Fix: `combat::exec` notes the row's motion when KH2 takes an Attack press
  (`attack_shown`); for 0.6 s after, opcode 322 run on Sora has its pushed value replaced
  with it (`probe::event_op`; the slot at `+0x198 + count * 4`, count `+0x194`, 0 for the first slot: the s145 build skipped 0 and never fired). Sora's own
  `+0x164` is untouched (KH1's attack objects tied to a motion compare it).
- Camera (`camera::jerk_note` summaries): the eye and look point move smoothly (≤ ~1 unit of
  change per frame walking) at 60 updates/s, but frames arrive 2..42 ms apart even standing
  still: uneven frame pacing, not the camera.
- The user's "slight slide" in the scene after "move and run": the log shows KH1's own scene
  motions there (slots 206 / 207, `follow` leaves KH1's animation): KH1's own walk.

### 2026-10-17: Stage 147 (the user's bug list)
- Attack on the first free frame after a KH1 move was KH1's attack, and KH1's combo ran on
  from there (out of order, glitched): `kh1_move` needed `active()` (KH2 drew a frame in the last
  150 ms), which lapses during any KH1 move longer than that. Now `HANDOFF_SEEN_MS` (the
  hand-off branch seen in `takeover`) keeps it; and `combat::exec` never runs KH1's attack row
  for Sora while `takeover::kh1_borrows` (his own body, no event / area script / ride / KH1
  movement mode / flight): it hands to KH1 with `ATTACK_AFTER`, KH2's first press when KH1 lets go.
- Confirm button on Circle (Options): KH1's pad step `0x140288F30` maps Cross to confirm
  (`0x210001`) and Circle to jump / cancel (`6`), swapped when `0x1402F4690` (a settings field;
  `+0x1A` Epic, `+0x1E` Steam) says so. We hid Circle from KH1 and read Circle as KH2's jump,
  so with the swap KH1 saw its jump and took Sora. `takeover::jump_button()` follows the option
  (pad mask and KH2's jump).
- L1 + jump cancelled KH2 moves when no shortcut was set: KH1 opens its shortcut menu on L1
  only then (`HAS_SHORTCUTS_RVA`), else the face buttons are its plain ones. The pad mask and
  KH2's `shortcut` now use `shortcuts::menu_on_l1`.
- Commands (magic, items, specials) cancelled any KH2 move: `combat::command` now runs one only
  when `can_cast` (spells: free, landing, an attack's ComboMagic window) or `can_take_command`
  (others); otherwise it waits (`PENDING_COMMAND`, KH1's 16-byte record) and
  `resume_command` (before KH1's control step) puts it back once allowed, within 1.5 s.
- Pause menu: KH1 poses a second copy of Sora's model there; any 293-bone instance was taken
  for Sora (one frame of KH2's pose, and KH2's walk at the menu Sora's zero speed: slow). A new
  instance is Sora only within 150 of his actor (`pose::at_sora`); the other is left to KH1 and
  noted (`menu_sora_within`). KH2 effects aren't drawn while it is (the menu's world pass drew a
  copy), and KH2's camera carries on across a pause (spamming pause restarted it unlocked).
- Keyblade trail: KH1 drops a trail asked for 30+ frames before the blow (the modifiers' long
  wind-ups: "not drawn" for motions 169 / 192 in the logs); asked again as the hit window opens.
- Aerial Finish (motion 194, combo row 29, ability 100) joins KH2's effects moves.
- KH2 effects step on KH1's frames too (`kh2fx::NO_MOTION`: the cut-short move's end); a spell
  mid-move froze them.

### 2026-10-18: Stage 148 (a Steam tester's log; the user's videos)
- A tester's log (Steam, s146 release): every pad record logged empty ("0000", sticks 80) the
  whole session, two record addresses alternating, while KH1 walked, jumped (`set motion slot
  9`, "KH1's own action, slot 12, KH1 state 1"), glided and ran its own air combo; only Attack
  and magic (KH1's command menu) reached KH2. Keyboard play: KH1 merges its keys in after the
  pad records (`0x140288F30`), which is all KH2 read. Their ini also had `commands = off` (KH1
  runs its commands itself, any time), `kh2_voice = on` (KH2's lines: the "wrong quotes"),
  `kh2_camera = off` (KH1's own camera, its own lock-on behaviour).
- `takeover::buttons()`: the pads' records or KH1's merged state (`0x14298D4A0 + 4` held: `6`
  as the jump button, `0x100000` as Square); `move_tilt()`: the pad's stick or KH1's processed
  one (`0x14298D4E0`). `probe::merge_pads`: every record seen in the last 100 ms merged.
- "KH2 has him" is now by game frame (`FRAME_KIND`, set each physics frame of Sora: KH2 / KH1's
  hand-off move / KH1), not 150 ms of time: a pause no longer drops KH2 (the battle pause's
  aerial combo went to KH1's table).
- KH1's control step (`takeover::control`) runs without its jump / cancel (`6`) and Square
  action (`0x100000`) bits while Sora is KH2's or KH1 has him outside one of its moves
  (`kh1_borrows`, no hand-off, no L1 shortcut menu): KH1's merged state, so the keyboard, every
  pad and the confirm swap alike; put back after (menus keep them). KH1 jumped on the first
  frame after a pause or an examine.
- Back from a command's hand-off (`FROM_COMMAND`): KH2's battle stance (a standing spell left
  him in the field idle).
- KH2's camera started while locked on faces the target at once (it came back unlocked after a
  pause, the battle pause, a limit, the save menu or examining).

### 2026-10-18: Stage 149 (command submenus, the spell queue, room entry)
- The command menu's current level: `0x142D54450` points at `&0x142D543F0` at the top level
  (there KH1's menu step `0x140291660` turns cancel, pressed bit `4`, into the jump flag
  `0x1000000` on the command record) and deeper in a submenu (magic, items, summons), where
  cancel backs out. `shortcuts::submenu_open()`: the pad mask, KH2's jump and the control-step
  strip leave Circle to KH1 then (the user couldn't leave those lists during a KH2 move).
- The command queue waits up to 5 s (a combo with its finisher; the user: a spell chosen
  mid-combo never came), keeps the menu step's own flags when it puts the choice back, and
  logs every wait / resume / drop on its own count (`cmd_log`).
- `pose::at_sora` compares a new model with Sora's actor from KH1's party slot
  (`takeover::sora_actor`); the probe's copy could be the last room's as a room loads, and
  his new model was refused (KH1's animations on entering a room, "sometimes").
- Logs: the last five runs are kept (`kh1-kh2.prev.log`, `.prev2` .. `.prev5`); the user's
  sessions were lost to two quick restarts with one kept. Reported, not yet understood: after
  sliding off a Traverse Town roof (KH1's own slide; the command menu greys), KH2's jumps stay
  low until a dodge roll (KH1's mover reads none of its jump fields `+0x3B0..+0x3D0`; the
  `ceiling:` log lines will say if it is our ceiling rule).

### 2026-10-18: Stage 151 (game volume, full ability lists)
- KH2's sounds follow the game's volume sliders: KH1's global settings (`0x14012AC50` returns
  them; Steam's build 4 bytes later) hold master `+0x10` (the device's, `0x14011BE4A`) and
  channels 1..3 `+0x12/+0x14/+0x16` (0..10; `0x1400DDD00` sets each channel to slider / 10), in
  the options menu's order: music, sound effects, voice. KH2's effects play at master × effects,
  its voices at master × voice, read when each starts (`kh2sound::game_volume`).
- A full ability list (the user's level 100 save had no Aerial Dodge): `growth::make_room` goes
  on past the least-missed list to any KH1 ability that's off (never `KEEP`: Guard, Dodge Roll,
  Combo Plus / Air Combo Plus, the specials, the abilities that are KH2 moves), then to the
  least missed even when on. Each still noted in `kh1-kh2-removed.txt`.
- Traverse Town's district doors ("curtain rooms"): KH1's animations after them, the room
  entry's refused model (stage 149's fix).

### 2026-10-18: Stage 152 (rooftop jumps; a tester's s148 log)
- Low jumps after a roof (the user's video; the tester's log: "ceiling: rose 0.0 of 7.5 at height
  620.1 (collision flags 00000009)" ten jumps running; the user's own logs the same at 457.2):
  KH1's collision bit 3 (`+0x2D0` = 9) is a slope's surface under him, not a ceiling (real
  bumps: flags 1, a partial rise). KH1's air mover pushes him along the roof rather than up;
  our ceiling rule took each jump for a head bump (cut it, locked air moves). With bit 3 the
  rise is KH2's own and no ceiling is called.
- The tester's log also showed: Sora's model refused mid-battle against the probe's stale actor
  ("drawn 872 away"), then "another body" and KH1's moves (stage 149's party-slot actor fixes
  it); spells waiting and dropped after 1.5 s (stage 149's 5 s); Aerial Dodge MAX not learned on
  a full list (stage 151). Pad records now counted once each.

### 2026-10-18: Stage 153 (no Zantetsuken, keep EXP Zero)
- The user: "remove zanteksukan, keep exp zero, increase the ability slot limit, have aerial
  dodge be its own ability still". Zantetsuken (KH1 id 59) is taken out of Sora's list like
  Superglide (the mod has no use for it; its level-up line is not shown), which frees a slot on
  a full list for Aerial Dodge (its own ability, id 67). EXP Zero (64) is in `KEEP` and no
  longer in `make_room`'s least-missed list; one taken out by stage 151 is given back (off).
- The list itself can't grow: KH1 keeps 48 bytes per party member at `+0x44`, and the next
  member's record starts `0x74` after this one (the save's layout, not ours).

### 2026-10-18: Stage 154 (the version in the Config menu)
- The user's friend: rooms breaking and no Aerial Dodge on a save that gets it here (this PC's
  s153 log: "Genie Jafar beaten: Sora learns Aerial Dodge MAX", AerialDodge motions after),
  matching the s148 build his earlier log came from. The user asked whether our addresses are
  absolute: they are RVAs (KH1 is loaded wherever ASLR puts it, `0x7ff6...` here), so they are
  the same on every PC with the same game build; an unknown build turns the mod off.
- `VERSION` (`lib.rs`, "s154") is shown on the second line of the Config menu's Camera Style
  help ("KH2 Combat version s154") and in the log's first line. `tests/version.rs` keeps it
  equal to the latest stage here: bump it with each stage. The README says how to check it,
  and that an update needs a Build again.
- Rooms in the log (the user: "the log should tell you whats happening when you enter a new
  room"): KH1's world and room (u32 at `0x142340E5C` / `0x142340EC4`; Steam `+0x233FE84` /
  `+0x233FE8C`, read from each build's `0x1402AFC10`, the flight-room check; KH-1FM-AP-LUA uses
  the same) are watched each frame: "room: world W room R entered" with who has Sora (KH1's
  reason when it does, his state, flags, a script), and 3 s on, how many of those frames KH2
  had him.
- Off a pole / bar (the user: KH1's falling animation after jumping off a pole): a KH1 move
  (`HANDOFF`) that ends in KH1's plain fall (state 2, above the floor, dropping; the log: bar
  state 15 -> 16 -> 2 with motion request 6, KH1's fall, -> 7 its landing) is KH2's from there:
  `Character::carry_into_air` starts KH2's fall at the speed KH1 had him moving (his position's
  change per frame, capped), so the swing's momentum and the fall's speed carry on.

### 2026-10-18: Stage 155 (footsteps, three moves' effects, landing hold, falls after spells)
- Footsteps mid-animation (the user): KH1 keeps Sora in its walk / run slots (5..7 in battle)
  whenever the stick is held, so its own footstep sound (`0x140293C70 (actor, kind)`, the
  floor's sound from `footstep.bin` by world `0x142340E5C` and material; Steam `0x140295E00`)
  played under KH2's attacks and rolls. Hooked: Sora's steps sound only while KH2 walks or runs
  him (or KH1 has him).
- Guard Break (165: effect elements 31, 30, 32 at frames 24, 28, 30), Finishing Leap (167: 33
  at 2, 34 at 28) and Aerial Recovery (40: element 0 at 0) join `kh2fx::DEFAULT_MOTIONS`;
  Aerial Recovery joins the sounds too. The sound log now names each element's sound, or says
  an element has none (Finishing Leap's sound, if not its elements', is still to find).
- Landing (the user's KH2 video: touchdown, ~16 frames of the landing crouch at 60 fps, then
  the run): `LAND_HOLD` 14 frames standing still before the stick moves him; buttons still from
  the reaction window. `docs/movement-model.md` corrected.
- Falling after KH1's magic in the air (the user: KH1's fall animation): KH1 drops him in its
  air state 1 there, not 2; once the spell is done (he can act) and he drops, KH2's fall as
  for stage 154's bars. KH1-move states are logged as they change.


### 2026-10-18: Stage 156 (spells in the air)
- s155 broke air spells (the user: "it cancels the spell and goes into the KH1 falling"). The
  log: a spell in the air (command 0x2 "in the air", KH1 motion 0x40000006) runs in KH1's air
  state 2 from its first frame, slot 80 the cast, sinking; stage 154's rule took state 2 +
  dropping as "the move ended in its fall" 10 frames in, set KH1's state to 0 mid-cast, and
  KH1's spell action then kept him (its fall slots 9..11) with KH2 never taking over. Now a
  command's fall is KH2's only once KH1 lets him act again (`0x1402956E0`), state 1 or 2; a
  grab's (bar, pole, ledge) as before. `release` logs a new reason even while KH1 already has
  him (that case left no line saying why).

### 2026-10-18: Stage 157 (falls handed over for real, air casts, pauses)
- Stage 154's handed-over falls never reached KH2 (the user's s156 log, a loop: "KH1's move
  ended in its fall: KH2's fall", then at once "KH1 has Sora (KH1's own action, slot 10)"):
  KH1's fall slot (10) is still up on the frame after, and `takeover` took a slot past 7 on a
  KH2 that wasn't airborne yet (no KH2 frame since) for KH1's own action. A fall just handed
  over (`AIR_START` / `HANDOFF_FALL`) counts as airborne, and KH1's jump / fall / land slots
  8..13 with KH1's ground state are leftovers, not KH1 actions (also: no more KH1 landing and
  idle before KH2 after a KH1 move).
- Air spells: KH1 keeps its cast pose and won't let him act until he lands (can-act false all
  the way down). A command in KH1's air state 2 whose current motion (not 8..13) has played its
  length (KH1 Sora's motion table, `pose::slot_frames`, at least 20 frames) counts as done:
  KH2's fall from there.
- Pause mid KH2 move (the user: resumed in a KH1 animation): `pose` drew KH2's last pose only
  while under 0.15 s old, so after a pause it was stale and KH1's slot was drawn; it is also
  used while Sora's last physics frame was KH2's (`takeover::kh2_frame`, game frames).
- Guard Break / Finishing Leap effects "look different" (the user's KH2 videos): offline
  renders (`fx_render` scenes guard_break, finishing_leap, aerial_finish, aerial_recovery
  added) show kh2-fx skipping programs 115 pppDrawMatrixWood, 120 pppRyjDrawMatrixWorld, 174
  pppVertexAp (and 49 pppVtMime for Aerial Finish): KH2's blade ribbons and Finishing Leap's
  white burst. Being implemented separately.

### 2026-10-18: Stage 158 (the guard's own motion)
- The user: our guard was "the animation he uses when somebody actually hits his guard" (with
  a KH2 video of the real guard). KH2's action button, standing, tries combo row 11 (exe
  `0x1403A9020` from the handler `0x1403A75A0`); in Sora's PTYA set row 11 is motion 173 (A322):
  the Keyblade's guard window ATKP 161 (refact self 1: a block starts the repel) frames 6..37,
  its trail 8..37, a swish sound at 1. Motion 42 (A321), which we played, is the repel (and
  Counterguard's window, row 39 while 42 plays: the same handler checks motion 0x2A). The press
  now starts row 11's motion (`motion::GUARD_PRESS` 173 as a fallback); a block still plays 42.
- `Output::weapon_trail` (the motion's trail window): KH1's Keyblade trail shows during the
  guard's swish; 173 joins the KH2 sound moves (its swish).

### 2026-10-18: Stage 159 (no KH1 high jump after its moves)
- The user (a video): whenever KH1 plays one of its animations, a jump on the first frame Sora
  can act is KH1's high jump. The log: an air spell, KH1 lands him (state 0, can act false),
  then state 1 with can act true at the floor (KH1 jumped), state 2, and "KH1's move ended in
  its fall (290 above the floor)". During `HANDOFF` the control hook left KH1 its jump / Square
  bits (KH1's own moves use them: jumping off a bar). Now once KH1's move has him standing and
  free (state 0 and KH1's can-act test, `kh1_move_done`) they are KH2's: stripped from KH1, and
  a jump press then ends the handoff with KH2 jumping at once (`JUMP_NOW` clears the held flag).

### 2026-10-18: Stage 160 (jump out of the landing hold)
- The user: the landing hold is right, but a jump should end it. A jump press now opens the
  landing's cancel at any frame (other buttons still wait for its reaction window, frame 8).

### 2026-10-18: Stage 161 (Guard Break's and Finishing Leap's full effects)
- kh2-fx gains the four programs those moves used and we skipped (read from KH2's exe;
  `docs/kh2-effects.md` §6, §7.3, §7.4, §12.1): 115 pppDrawMatrixWood (an upright billboard,
  turning to the camera about the effect's own y: Guard Break's spiral around the thrust),
  120 pppRyjDrawMatrixWorld (an instance keeps the root it was spawned at: Guard Break's light
  fans stay put while he lunges), 174 pppVertexAp (VertexApLc with the vertex taken through the
  spawner's matrix: Finishing Leap's five flame spikes on a ring around him), 49 pppVtMime (a
  vertex morph between two models: Aerial Finish's rings widening). Offline renders
  (`fx_render`, new `--flip-z` / `--follow-root` options) skip nothing for guard_break,
  finishing_leap and aerial_finish; explosion and magnet render byte-identical.

### 2026-10-18: Stage 162 (jump casts drawn as KH2's, camera, Guard Break's spiral)
- Jump casts still showed KH1's animation all the way down (the user; the s161 log: a spell in
  the air stays in KH1's air state 2 with its cast slot 80 until he lands, so stage 157's "cast
  played through" never came before the ground). Now drawn, not taken: while KH1 has him for a
  spell cast in the air (`takeover::air_cast`, the spell from the command's second byte: Fire 0
  .. Aero 6), `pose` plays KH2's air cast for that spell (MAGC air motions 58 / 61 / 64 / 67 /
  70 / 73; Gravity Magnet's, Stop Fire's, Aero Reflect's), then KH2's fall, and KH2's landing
  once KH1 has him on the ground. KH1 still moves him and runs the spell.
- Out of battle, KH1's hold at the end of such a landing counted as a "scene" and gave the
  camera to KH1's (the user: a jump cast of Fire zoomed the camera in). Holds while or within
  1.5 s of one of KH1's moves no longer count.
- Guard Break's effect sat around the middle of the Keyblade (the user). Its spiral (element
  32) is 235 along -z of KH2's object root; our root frame mirrored the model's y only, which
  keeps the model's forward (+z) and so put the spiral behind him. The root frame is now the
  model's turned half round about x (y and z negated); centred elements (Explosion, Magnet
  Burst) are unchanged.

### 2026-10-18: Stage 163 (KH1's jump casts, landing hold after them, L1 spam)
- The user: KH1's magic animations after all (stage 162's KH2 air casts taken out), but the
  landing after a jump cast should have KH2's landing hold, and a jump out of it KH2's jump.
  When KH1 lands him out of an air cast (`air_cast`, KH1 state 0 on the floor), KH2 takes him
  at once in its landing (`Character::land_now`: the 14-frame hold, a jump at any frame); KH1's
  cast action flag (`+0` bit 4) is cleared so its can-act test lets KH2's input through.
- Spamming L1 and jump launched Sora in KH1's animations (the log: KH2's jump, then KH1 state 2
  in its jump slot 12 under KH2, "KH1's own action, slot 12/13", state 21). The control hook
  left KH1 its jump whenever the raw pad had L1 (KH1's shortcuts); KH1's own button state can
  differ from the raw pad for a frame, and then saw a plain jump. The strip now follows L1 in
  KH1's own state (`0x40000`, its pad table `0x140528F90` maps the raw `0x400` to it).

### 2026-10-18: Stage 164 (jump held, then L1)
- The user: holding jump and pressing L1 "does crazy stuff". The log: KH2's landing with KH1
  in its state 2 and jump slot 12 (flags 0x48003), then KH1's own air moves (state 21). With
  L1 down (KH1's shortcuts) the control hook left KH1 all its button bits, the held jump too,
  and KH1 jumped him as KH2 landed. In the shortcut menu (or a submenu) only KH1's "pressed"
  bits stay now (a shortcut, backing out); the held and repeat jump / Square bits are KH2's.

### 2026-10-18: Stage 165 (air casts play through; KH1's own jump under KH2)
- The user (a video): Fire in the air "isn't playing the full animation and looks weird": the
  log showed stage 157's "cast played through" handing a jump cast to KH2's fall mid-air. Air
  casts (`air_cast`) now stay KH1's to the ground, then KH2's landing (stage 163).
- Holding jump and pressing L1 still launched him (the s164 log: "L1, Circle" while KH2 had
  him in the air, then KH1's jump motion (slot 9, set from its own state step, not the control
  step we strip around), and KH1 state 2 / slot 12 as KH2 landed). Wherever it comes from: when
  KH1 has put him in its jump (state 1 / 2, slot 8 / 9 / 12 / 13) on a frame KH2 had him and
  no handoff ran, KH1's state goes back to 0 and KH2 keeps him (logged).

### 2026-10-18: Stage 166 (no jumping through Olympus's roof)
- The user: in the Coliseum's lobby a jump went through the roof. The log: "rise on a slope's
  contact (flags 9)": that ceiling carries KH1's collision bit 3 too, and stage 152's roof rule
  let KH2's rise through. Bit 3 now counts as a roof only when it was there from the first 3
  frames of the rise and unbroken since (`RISE_FRAMES`, `SLOPE_RUN`): taking off from a roof,
  not meeting a ceiling on the way up.

### 2026-10-18: Stage 167 (attack out of a spell's end frames, combo carried on)
- The user: KH2's magic has end frames an Attack cuts into, going on with the combo. KH2's
  ground casts open their range-14 window about 8 frames after the magic-cast event (Fire 56:
  cast 22, window from 30 of 80; Blizzard 59: 16 / 24 of 72; Thunder 62: 18 / 26 of 68).
  For KH1's casts (`HANDOFF` from a command): an Attack pressed meanwhile (`ATTACK_AFTER`)
  now ends the handoff only once the spell is out (`spell_out`: a new attack object owned by
  Sora in KH1's pool, `combat::sora_shots`, or 55% of KH1's cast motion for spells with none),
  not before (before, an Attack 10 frames in could cut the cast itself). KH1's action flag is
  cleared there, and KH2's combo state from before the spell carries over
  (`Character::combo_state` / `set_combo_state`: the next swing counts on from it).

### 2026-10-18: Stage 168 (animation-cancel KH1's spells)
- The user: "animation cancel the KH1 spells". Once the spell is out (`spell_out`), not only
  an Attack (stage 167) but a jump, Square (roll, guard, glide) or the stick ends KH1's cast
  and hands Sora to KH2 at once (KH1's action flag cleared; logged "spell's end frames
  cancelled by ..."). Before it is out, Square and the stick no longer end the handoff either
  (they could cut the cast 10 frames in). Only an Attack carries the combo on.

### 2026-10-18: Stage 169 (pausing mid KH2 move, again)
- The user: pausing during KH2 animations still put him in KH1's. The s168 log: paused while
  falling (KH2's Air), resumed, "KH1 has Sora (Sora is in another body)", KH2 restarted
  standing in mid-air, KH1's fall slot took him (state 21, its air moves). `sora_body_seen`
  timed Sora's drawn model by the clock; during the pause only the menu's copy is drawn, far
  from his actor. A gap of more than 250 ms in Sora's physics frames marks a resume
  (`takeover::resumed_within`), and for 800 ms after one his body counts as seen.

### 2026-10-18: Stage 170 (Reflect in the Magic menu)
- The user: "lets add reflect into the magic menu, i want the sound and VFX". KH1's command
  records (`0x14028D7D0 (id)`, 16 bytes, 256; 0x7D.. empty) get 0x7D: magic, spell byte 7,
  word index 0x1FF (named "Reflect" by pointing the name drawer's `0x14028D820` call at
  `0x1402787FB` to ours). After KH1's battle-menu fill (`0x140292330`) it goes into the magic
  list (`0x140529638`) before Summon once Sora has Aero (`reflect::learned`), greyed as KH1's
  spells are with Fire's cost (`0x1402A2A80`; spell data `0x142D23598 → +0x30`). The list's
  layout (`0x142859E60`: `+0x10` / `+0x28`) gets 8 row panels for 9 entries (KH1 builds 7).
- `combat::command` hands command 2 / spell 7 to `reflect::chosen` (KH1 never runs it). KH2
  casts it when its move allows (MAGC: 71 / 72 / 73), pays MP (`0x1402A39D0`), and runs
  `magic/REFLECT_1.mag`'s script with `kh2-magic` (trigger 20, release at frame 0). While ATKP
  573 lives, `hurt` stops Sora's hits (no-effect) and counts them. The counter (574 every 10
  frames at level 1, then the 667 burst) lands through `Strikes::spell_strike`.
- The spell's PAX is drawn beside Sora's package (`kh2fx`, textures appended), its elements'
  sounds play (`kh2sound`), cast voices only with `kh2_voice`. ini `reflect = on`. Not in the
  shortcuts (KH1's shortcut slots hold spells 0..6). `docs/kh1-reflect-spell.md`. Built by a
  helper session from s166 and merged over s167..s169.

### 2026-10-18: Stage 171 (no cancelling jump casts)
- The user: "you can animation cancel out of KH1 jump spells, so no good there". Stage 168's
  end-frame cancels (Attack, jump, Square, the stick) now apply to ground casts only; a cast
  in the air (`air_cast`) plays KH1's animation through to the ground and KH2's landing hold
  (stage 163).

### 2026-10-18: Stage 172 (Reflect in the shortcuts, Reflera / Reflega, 1 MP, held barrier, effects under the pause menu)
- The user: "the reflect vfx pass through the pause menu, make reflect able to be put in the
  shortcuts, also include the two upgrades. Reflect should cost 1 MP", then "can we have
  reflect keep shielding until attacks have stopped". Built by a helper session from s170.
- Pause menu: KH1's pause menu task (`0x1402D3040`, priority 159999, opened by `0x1402D3450`)
  draws its windows before the world pass and repoints the camera for the party models, so our
  effects drew over it. `kh2fx` draws nothing while KH1's pause state `0x14232E9B4` is non-zero
  (1..6) or Sora has had no physics frame for 120 ms (`takeover::paused`).
- Shortcuts: only KH1's battle fill reads the slot bytes and indexes its spell table by them,
  so Reflect is never stored there: its slots keep KH1's none (`0xFF`) plus a bit in `+0x847`
  (bits 4..6; the fourth's nibble is 8). After the fill those entries are command 0x7D (the
  menu's path, `reflect::chosen`); KH1's learn sees them as taken.
- Customize: the spell list (`0x142E9A9F0`, 8 ints) moves to a buffer of ours (its four `lea`
  repointed) so Reflect fits before "none"; 9 rows 25 apart. Its help line (`0x14030028E`) and
  name calls (`0x14028D100`) are ours. Works with `shortcuts = 3` too.
- Levels: `reflect::level` = Aero's level (KH1's spell table `+0`, else save `+0x492 + 6`);
  each level its own `REFLECT_n.mag`, MAGC row and sub; Reflect / Reflera / Reflega.
- MP: KH1's MP is whole points; a cost ≥ 100 is hundredths (check `cost/100 <= MP`, pay
  `cost/100`), so Reflect costs 100 (1 MP) at every level.
- Held barrier: from the barrier's 22nd frame, while the last blocked hit is ≤ 20 frames old
  (cap 5 s), `kh2_magic::Magic::hold` freezes the cast (script and effect ages; its attacks
  stay, so `hurt` keeps blocking); the counter fires after; the barrier effect is restarted if
  it ends. Tests `reflect_counters_what_it_blocked`, `reflect_barrier_held`.

### 2026-10-18: Stage 173 (glide in place)
- The user: "In KH2 you're able to glide in place". The glide's speed now follows the stick's
  tilt (`(1 − a)·glide_speed·tilt + a·|v|`): with the stick let go he slows to a hover where he
  is, still descending to and settling at `glide_fall_height`. The exact KH2 scaling is not
  traced in the exe (hypothesis in `docs/movement-model.md`).

### 2026-10-18: Stage 174 (Reflect falls in the air)
- The user: "make sora fall while casting reflect mid air, just like kh1 magic". KH2's air cast
  (motion 73) hovers in its no-gravity window (range id 3, frames 16..74). `Character::
  air_magic_falls` (the hook sets it) skips that window for magic actions: the cast falls from
  rest as it plays; landing keeps the cast going on the ground.

### 2026-10-18: Stage 175 (Reflect spammed: one counter at the end)
- The user: "reflect needs to not do the burst damage until the player stops spamming it".
  A Reflect cast while the last one is still in flight (barrier, hold or counter) now ends that
  one at once (`kh2_magic::Magic::cancel`: scripts stop, effects killed, attacks freed) without
  its counter, and the hits its barrier counted carry over to the new barrier
  (`Runtime::carry`, added to its reflect count once it stands). Only the last cast of a chain
  fires the counter and burst, for every hit blocked along the way.
- Stage 174 (Reflect falls in the air) went out in the same build.

### 2026-10-18: Stage 176 (magic in the combo's queue)
- The user: "reflect and the other magic should be part of the combo cue, so it can get cast
  and the next attack waits for the magic to get done", and "reflect should have a finisher
  like in KH2". KH2's casts (Reflect): an Attack pressed during the cast is buffered
  (`buffered_attack`) and taken when the cast's reaction window opens or at its end; a cast
  started mid-combo leaves `combo_carry`, so that swing goes on with the combo. KH1's casts:
  an Attack waiting through one (`ATTACK_AFTER`) now carries the combo on at the handoff's end
  too (`COMBO_ON`), not only when it cut the end frames.
- Reflect's finisher was already there (KH2's rule, `Character::cast`: cast in a combo-magic
  window when the next swing would be the combo's last, MAGC finisher motion 72, sub + 1);
  the user's logs only show casts from free movement (shortcuts), where it can't happen.

### 2026-10-18: Stage 177 (Horizontal Slash in the combo; air casts land into KH2's landing)
- The user: "Horizontal slash can be spammed forever, this is bad". A forced Square move row
  (Upper Slash 12, Horizontal Slash 38) only checked the row's own conditions, and a normal
  row has no count check, so Square kept taking Horizontal Slash (a jump row: each one hops,
  so Sora stayed up). Now a non-finisher Square move needs `count < max` after the count
  update: it is one swing of the combo, and at the end Attack's finisher is next.
  Test `horizontal_slash_is_part_of_the_air_combo` (24 slashes before, 1 in a combo of 3).
- The user: Final Form's air Reflect falls and lands during its animation; mimic it for base
  Sora and end in KH2's landing. The fall was s174 (`air_magic_falls` skips motion 73's
  no-gravity range); now a falling air cast touching the ground goes into the Land action
  (with its 14-frame hold, jump-cancelable) instead of standing out the rest of the cast.
  The barrier and its counter don't watch the caster's motion (kh2-magic test with the cast
  motion ending at frame 8: same barrier and counters), so landing early keeps Reflect up.
  Test `falling_air_cast_lands_into_the_landing`.

### 2026-10-18: Stage 178 (glide hover, Aerial Dodge, Reflect's sounds, Fire's cancel, magic in the combo)
- "KH2 sora has a animation for gliding in place": KH2's ACTION_GLIDEHOVER (exe `0x1403A8F44`:
  a glide started with the stick neutral is action 0x11, else 0x12; `0x140422B20` plays motion
  208 and goes to the glide on the stick; `0x140422930` slows it ×0.9 a tick). Ours: the same,
  and a glide let go hovers once slowed under 2 per frame. `motion::GLIDE_END` (208, unused)
  is `GLIDE_HOVER`. Test `glide_in_place_hovers`.
- "aerial dodge doesnt push you forwards unless you're tilted forwards in KH2": neutral, the
  dodge has no horizontal speed (was `air_dodge_speed` along the facing). Test
  `neutral_aerial_dodge_goes_straight_up`.
- "These are the wrong Reflect sound effects": se000's id records are 68 bytes and the u16
  read as the entry (end - 4) is only a running count there; the entry is the last u16 (end -
  2), equal to it in se999 / se500 (20-byte records, the sounds checked before). Read that way
  every spell's ids land in one block (Reflect's on entries 180..187) and 12 of 124 ids are
  unplayable (57 before; Reflect's 138 / 139 / 176 played nothing). Fire, Magnet Burst's 169
  and the other se000 sounds change too. `docs/kh2-sound.md`.
- "can you make it so fire cant be cancelled out until fire actually shoots the projectile":
  `spell_out` no longer falls back on 55% of KH1's cast motion for spells with a shot (only
  Cure / Stop); the others wait for their shot (or KH1's whole cast motion). The shot test
  compares with the fewest attack objects seen since the cast (a swing's hitbox out at the
  handoff ended during it, so Fire's shot came back to the start count). Logs "command's spell
  Some(n) out after t s (its shot / KH1's cast motion's time)".
- "casting a spell mid combo ... should essentially add onto the main combo / it should insert
  itself" (video: Reflega mid-air combo, then a whole new combo): a spell is mid-combo while a
  swing plays or one just ended (in the air until landing, on the ground for 60 frames), not
  only during a swing (`combo_live`); it takes a step (counted as `try_attack` counts: hit,
  Combo Master or in the air) and the combo goes on after it. KH1's spells too
  (`Character::magic_step` at the command handoff, `SPELL_COMBO` restores the combo after).
  Test `spell_mid_combo_is_a_step_of_it`.

### 2026-10-18: Stage 179 (revenge value, Reflect's invulnerability, Attack cuts air casts)
- "multi-hit attacks (like explosion) should only have the last hit towards revenge value". KH1
  has no revenge value in its engine (traced by a helper session: no counter; a hit makes a
  record queued on the foe, `+0x3EC` linked by handles at `+0x30`, with the foe's hit flags
  `+0x3E0` and serial `+0x3E8`; a foe with a script (`+0x374` bit `0x10000`, machine `+0x144`)
  hears of it at once, event 2 from `0x1402BCF20` in the contact, and again when it takes its
  queue, event 0 from `0x1402B59F0`; its script applies the damage and counts hits as it
  likes). KH2's own data marks the hits: revenge value 0 on Explosion's ticks (911) and 24 on
  its blast (912), Magnet Burst 1901 / 1903, Aerial Finish 915..917 / 927. So a hit whose row
  has none while one with some is still to come in the motion (`revenge_to_come`) is landed
  silently on a scripted foe: script flag off for the contact, the record taken back off the
  queue and freed, flags and serial restored, its whole damage carried (`CARRY`) to the foe's
  next hit, normally the move's last. Unscripted foes as before. Logs "silent hit on scripted
  foe". The ticks' flinch is the script's too, so scripted foes no longer flinch to them.
- "while the reflect animation is up sora should be invincible all the way through": enemy hits
  in any of Reflect's cast motions (MAGC ground / air / finisher of each level) are evaded
  (`hurt`), before the barrier stands and after it falls; the barrier's own still count.
- "magic transition into the base attack during a combo" (asked: "Attack cuts the spell
  short"): an Attack once a KH1 spell is out now cuts air casts too (stage 171 left air casts
  uncancellable; jump, Square and the stick still don't cut them), into KH2's next combo swing.

### 2026-10-18: Stage 180 (magic out of a swing at once; Reflect chains in a combo)
- "too much a pause before magic gets cast in a combo" and "make it possible for attacks to
  cancel into magic": Sora's swings have no magic window (only the casts have `ComboMagic`), so
  `can_cast` was false through a whole swing and a spell chosen mid-swing waited for its end
  (0.15..0.7 s in the user's log). A swing now takes a spell in its combo or finisher window,
  when it would take its next swing (right after its hit). Test
  `spell_cast_from_the_swing_combo_window`.
- "if sora does a combo with reflect, any attack done to every reflect should burst out of the
  final reflect in the chain": a Reflect that blocked something waits with its counter (held,
  barrier hidden and stopping nothing) while the combo has steps left (`Output::combo_live`);
  a later Reflect in the combo takes its hits over (s175's chain), the combo's end lets the
  last one burst for all. `docs/kh1-reflect-spell.md`.

### 2026-10-18: Stage 181 (no footsteps between swings)
- "i can still hear the footstep sound fx while walking forward in the middle of a combo":
  stage 155 let KH1's footstep through on any frame KH2's state was Walk / Run; between two
  swings with the stick held KH2 is in its run for a few frames (still showing the swing, the
  log's "Run (motion 166)"), and KH1's run cycle stepped then. Now only after `STEPS_AFTER` (10)
  frames running KH2's own walk / run motion (1 / 2).

### 2026-10-18: Stage 182 (Reflect chains burst for real; the burst's sound)
- "if reflects are cast into one another, basically delay the burst until the last reflect, all
  attacks and triggers should burst at the last reflect". The user's s180 log: "cast again in
  flight ... 2 blocked hit(s) carried over", "2 carried hit(s) counted on the new barrier
  (count 4)", then "2 hit(s) blocked, 0 counter hit(s)": `Magic::cancel` left the cancelled
  cast's attacks standing until the next `update`, so the carry went onto the old barrier
  (still found first) and the new one countered for nothing. The chain (s175) never burst.
  `cancel` now frees the cast's attacks at once (kh2-magic test
  `reflect_chain_counters_from_the_last_cast`, which fails without it). The carry is also
  every hit of the chain so far (`rt.blocks`), not only a standing barrier's count, so a Reflect
  cast into a counter already under way carries its hits too.
- "the burst sound isnt in the game either": the script starts no element for the burst
  (ATKP 667 / 668 / 669); KH2's own hit effect for it (`trap_attack_set_pax`) isn't modelled,
  and element 11's sound (174) plays nothing in the PC files. Element 7's sound (173, shared by
  all three levels, 1.9 s) now plays as the burst attack appears. Inferred, not traced.

### 2026-10-18: Stage 183 (the user's magic-combo spec; aerial landing, submenu guard, roll-off dodge, dodge momentum)
- The user's written spec (magic in the combo string, KH2 Reflect finisher rules, KH1 magic
  integration, three combat bugs) and a bug report (Aerial Dodge momentum). In the controller
  one rule for a spell's combo slot (`spell_slot` / `take_spell_slot`, `SpellSlot`), used by
  KH2's casts (`cast`) and KH1's spells (`magic_step`): a new combo's first step, the next
  step of a going one (its finisher on the last), or during a finisher only the next finisher
  while Finishing Plus leaves one. `can_cast` follows it (a finisher's window only for the next
  finisher; nothing after the last finisher until it ends). A finisher cast (`cast_finisher`)
  takes no cancel except the next finisher in its finisher window, and ends the combo; a
  finisher swing's finisher window cancels only while another finisher may follow.
  `try_attack` takes a finisher cast (or a spell that left `carry_finisher`) as the finisher
  playing. `ComboState` carries `finisher_next` over KH1's casts. Tests: `reflect_cannot_
  follow_the_last_finisher`, `finishing_reflect_as_the_first_finisher_then_a_finisher`,
  `finishing_reflect_twice_then_the_combo_ends_after_it`, `finishing_reflect_as_the_second_
  finisher`, `reflect_mid_combo_is_one_step`, `kh1_spells_take_one_slot_each`.
- KH1 spells in the hook: the slot is logged ("spell cast: combo slot n") and decides what may
  cut the cast once the spell is out (`SPELL_SLOT`): an Attack or another spell (the combo's
  next step) unless it was the combo's last slot; a jump or Square only out of a step on the
  ground; never the stick (spec rule D). KH1 runs no command during its cast, so an Attack or
  a spell chosen then is taken off its command record (`combat::take_chosen_command`; the user:
  no attack out of magic); a spell waits in the queue (`MAGIC_AFTER`, `resume_command` keeps
  it through the cast). After the cast the combo goes on from the spell's slot (not after the
  last). A Reflect barrier waits for the combo only with something else in it (count ≥ 2).
- Spec 3.1, aerial attacks to idle: the user's logs, "Attack 181 at 85 above the floor -> Run",
  "Attack 192 -> Idle": landing mid air-attack went to Free; now KH2's Land. Test
  `air_attack_lands_into_the_landing` (fails without).
- Spec 3.2, guard from a submenu: with a command submenu open the pad mask and the control-step
  strip left Square to KH1, whose own guard works in the air. Now only the L1 shortcut menu
  leaves KH1 Square; a submenu keeps only its back-out (jump / cancel), Square is KH2's (its
  guard ground-only).
- Spec 3.3, no Aerial Dodge after rolling off a ledge: `air_dodge_ready` came only from a jump;
  a roll or Quick Run taking off (`since_roll`) gives it too. Test
  `aerial_dodge_after_rolling_off_a_ledge` (fails without).
- "Aerial Dodge momentum is reset ... when the player presses a directional input": air control
  steered toward the run speed (8) at `air_keep` 0.9 a tick, draining a dodge's 18..36 a frame
  in a few frames, while letting go only applied `air_drag`. Now the stick steers without
  fading the speed faster than `air_drag`, unless pushed against. Test
  `dodge_momentum_kept_with_the_stick` (fails without).
- Review (a separate session, against the spec) found three more, fixed: a finishing spell
  could follow a finisher that missed (`spell_slot` and `can_cast` now need the finisher to
  count, as `try_attack` does; test `no_finishing_reflect_after_a_missed_finisher`); an air
  spell's combo (its finisher carry) survived the landing of Reflect's air fall (`land()` resets
  the combo there, as for aerial swings; test `air_spell_combo_ends_on_landing`); a spell chosen
  after KH1's cast had ended waited in the queue until the stick or a button moved
  (`kh1_casting` now needs Sora still unable to act, and a spell waiting once the cast is done
  ends the hand-off). Known limit: KH2's air cast motion (73) has no finisher window, so after
  an air finishing Reflect the next finisher can't follow in the air.

### 2026-10-18: Stage 184 (multi-hit moves flinch again)
- "Multi-hit moves should continue triggering the enemy's taking-damage animation on each
  appropriate hit, while revenge value is only added on the finishing hit": s179's silent hits
  never reached the foe's script, which is what plays its reactions (its reaction calls run
  KH1's engine reactions, `0x1402B9B60` → `0x1402B62C0 (actor, record)`; the default path
  `0x1402BCD30` only changes HP). Now a silent hit on a foe already in one of its damage motions
  (`+0x164 - +0x3EA` under 10, KH1's own test in `0x1402B62C0`) runs that engine reaction itself
  (and `0x1402B65F0` first for records with `+0xC` bit `0x800`, as the call does) before the
  record is freed; a foe not reacting hears the move's first hit as usual (its script decides
  whether it flinches: super armour stays), the rest stay silent (`TOLD`, per move and foe).
  So a multi-hit move counts at most twice toward a script's hit count (its first hit when the
  foe wasn't already reeling, and its last), never once per tick; forcing a flinch the script
  didn't choose risks breaking bosses (the Cloud kind of bug). Damage still carried to the last.

### 2026-10-18: Stage 185 (Keyblades teeth up; hair, hood and keychain physics)
- "sora holds his keyblade backwards, with the teeth of the keyblade facing the ground", "every
  keyblade is held backwards": the grip bone (280) hangs from the hand as in KH1's rest pose,
  whose roll of the blade isn't KH2's. The blade and keychain lie on the grip's x (keychain
  bones at -21..-56 on it in the bind), so every Keyblade (not 0x52..=0x55, the dream weapons
  and the wooden sword, nor `flip_weapons`) is turned half round about that axis with its
  subtree (`pose::keyblade_teeth_up`; ini `keyblade_turn = on`).
- "sora's keychain, hair, hood etc dont have wind physics during any KH2 moves": KH1 has none
  either (each of its motions animates them by hand), so under KH2's motions they stayed rigid.
  `kh1_data::jiggle`: per chain, particles in world space pulled back toward the pose, with
  gravity and a breeze, kept at the bones' lengths, each bone turned at its simulated child
  (subtree with it): the 14 hair spikes under the head (151; root, joint, tip each), the hood
  80..84, the keychain 284..292. Reset after half a second without KH2's pose or on a warp.
  ini `physics = on`. Tests in `jiggle.rs` (still pose stays, a carried chain trails then
  settles, a warp restarts it). Tuning is a first guess: not yet seen in game.

### 2026-10-18: Stage 186 (physics toned down)
- The user's video of s185: "it works, but ... needs to be toned down a lot, there's a lot of
  clipping" (hair spikes splayed into each other and the head, the hood flipped up). Stiffer
  springs and more damping, no gravity on the hair and little on the hood, a breeze a tenth of
  s185's, and each bone's swing capped from its posed direction (`Chain::max_turn`: hair 10°,
  hood 12°, keychain 35°; the simulated tip is pulled back to the cap so the swing doesn't build
  past it). Test `swing_capped`.

### 2026-10-18: Stage 187 (physics steps by game frame)
- "the animations arent smooth, they kind of stutter" (s186 video). The jiggle stepped by the
  clock in fixed 1/60 s steps; frames come 13..22 ms apart (the camera's log), so some draws ran
  no step (the chains left at Sora's last place, then snapping after him) and others two. Now it
  steps once per Sora physics frame (`takeover::GAME_FRAME`, counted where `FRAME_KIND` is set);
  a draw of the same frame again re-applies without stepping; over 30 frames missed, it starts
  over. (The recordings' whole-frame repeats, 16..35% of frames, are in older videos too: the
  recording or frame pacing, not ours.)

### 2026-10-18: Stage 188 (the keychain hangs)
- "the actual movement of the keychain is good, but ... the starting position of it doesnt look
  natural, it looks like the chain is kind of floating": KH1's rest pose holds the keychain
  straight out of the pommel along the blade, and s186's pull back there (stiffness 0.18) with
  its 35° cap kept it out. Now a free pendulum (no pull, full gravity, damping 0.12, 170° cap):
  it rests hanging from the pommel. A chain starting over first settles (`SETTLE` 120 steps, no
  breeze), so it isn't seen dropping into place. Test `a_free_chain_hangs_down`.

### 2026-10-18: Stage 189 (Drive: the second command page)
- The user: "add a drive form. For now, lets add wisdom form ... press left on the dpad to have
  essentially a second command menu (like how it is in KH2,) and have a drive menu at the bottom
  of the second command menu". Asked: Wisdom costs MP; KH1 Sora recoloured blue; page 2 is
  Attack, Magic, Items, Drive. Three helper sessions researched the menu, Wisdom's data and the
  recolour; this stage is the menu (`docs/kh1-drive.md`, `win/drive.rs`, ini `drive = on`).
- KH1's top menu (menu 0) is rebuilt each frame; its fourth row isn't one of its entries but
  the magic list's last (Summon, command 6, opening menu 3). Page 2 turns that row into Drive:
  D-pad left (logical `0x80`, cleared before KH1's step, which would move the cursor to Attack)
  at the top level flips the page (not with the L1 menu, a submenu or a summon out); after the
  fill the magic list ends in Summon (appended when KH1 had none), its bit is Drive's, the top's
  Magic entry is forced in when Sora has no spell (blocked), menu 3 holds our form commands
  (records 0x7E.., magic type, argument 0x10 + form, word 0x1FE..), and word 6 reads "Drive"
  on page 2. A form chosen reaches `combat::command` as magic with that argument
  (`drive::chosen`; this stage only logs it and plays KH1's "can't" sound). The step, fill and
  name hooks are shared with `shortcuts` / `reflect` (whichever installed them calls `drive`).

### 2026-10-18: Stage 190 (the keychain hangs down)
- "the keychain is now floating upwards??" (a screenshot): gravity pulled toward the world's -y,
  but KH1's world has y pointing down (the pose's model space is y-up; `to_world` flips it), so
  the free keychain of s188 rose. Down is now the model's down taken through `to_world`. Test
  `hangs_down_in_a_y_down_world`.

### 2026-10-18: Stage 191 (Wisdom Form; lunges kept out of exits and crates)
- Wisdom Form itself (`win/form.rs`, `docs/kh1-drive.md`): page 2's Drive entry, 3 MP (KH1's
  own payment), KH2's change (Sora's Drive motion 52, invulnerable; the swap to `P_EX100_MAGF`
  loaded beside KH2 Sora, `pose::Form`, facing / lock kept; arrival 53), Quick Run LV1 and
  Sora's support abilities, 40 s of battle time (the gauge stands still out of battle), the
  entry reading "Revert" in the form, revert motion 4 on the ground, at once on a KO (`hurt`).
  Its shots: the motions' frame trigger 1 runs the model's `wisd` script (`Magic::
  object_effect`) with its effect archive as `kh2fx` package 4; the shot's unbound effect is
  placed where the script moves it (new kh2-fx `Effects::set_anchor`); ATKP 859 lands through
  the spell hit records (now returning the attack key, so the hit is passed back to the
  script); unlocked, the nearest enemy in front is the target. Reflect's sub + 10 in the form.
  Reflect's script world is shared (`reflect::world_now`).
- The blue (`win/tint.rs`, `docs/kh1-tint.md`): a hook on KH1's mesh draw `0x1401DB7F0` sets the
  mesh shader's Ambient / fog for Sora's clothes meshes (3..7: jacket, jumpsuit, trim), read
  from his model's mesh table, and puts KH1's back after; faded in over 15 frames.
- "a combo pushing you forward shouldnt bring you into a new room if you're by the entrance of
  a room, also, if a combo pushes you forward into a box, it shouldnt go into soras push
  animation" (`win/gates.rs`, `docs/kh1-exits-push.md`): exits are collision faces with an
  event number that KH1's mover queues on every touch; during takeover's move of an attack
  frame a new face event is dropped and the move undone. Crates count 12 frames of Sora's
  touch themselves; during KH2's attacks (and 20 frames after) their "touched by" field is
  cleared before the count.
- None of it watched in game yet.

### 2026-10-18: Stage 192 (Wisdom: its spells, slide, moving shots; tint and effect fixes)
- The user: "give wisdom form max quick run, KH2 sora slides when he stops in wisdom form. Also,
  lets use the wisdom form version of these spells [asked: Fire, Blizzard, Thunder, Cure]. Also,
  using the base wisdom form combo shouldnt stop sora in his tracks"; "sora's outfit isnt blue,
  and the effects are still covering sora"; "the effects last after reverting as well".
- Quick Run MAX in the form. `character-core`: `MoveParams` acceleration / brake (from prty)
  and the allow-movement range (41), tests `ground_inertia_slides_to_a_stop`,
  `attack_moves_in_its_allow_movement_range` (`docs/movement-model.md`).
- `formmagic.rs` (the shots moved there from `form.rs`): KH2's Fire / Blizzard / Thunder / Cure
  in the form (KH1's menu or shortcut → `combat::command` → `spell_chosen`; cast when KH2's
  move allows, `before_step`), their packages in `kh2fx` (handles now carry 5 bits of package);
  Cure heals KH1's HP. Test `wisdom_spells_run_at_their_sub`.
- The light on Sora was the arrival's elements (100 loops until ended): ended when the arrival
  does; everything of the form's put out on a revert.
- The tint found no table: the PC's Sora model uses the alternate mesh table (`0x1401DB4F0`);
  read now, pieces by texture index; the log lists the table.
- The gauge drains ½ a frame out of battle (KH2's), 1 in battle.

### 2026-10-18: Stage 193 (the Drive glow ends; no magic queue; revenge changes reverted)
- "the glow still covers sora in the form": Sora's Drive motion (52) starts his package's
  elements 2..13 on his limbs, looping (3..6) until ended; flagged 0x200, they end with their
  owner, which KH2 replaces at the swap. kh2-fx `Effects::owner_gone`, `kh2fx::owner_gone` at
  the swap and the revert.
- "fire and blizzard dont work in wisdom form": the cost read from KH1's spell data (`+0x30`)
  was 30 for them (3000: refused) and junk for the others; every KH1 spell costs 1 MP.
- "remove [the magic queue] entirely but ensure that magic stays connected to the base games
  combo strings": a command, Reflect or a Wisdom spell chosen during a KH2 move now waits only
  while that move plays (`takeover::KH2_MOTION` / `same_move`), going at its next combo window
  or not at all (KH2's own buffer), instead of up to 5 s. A spell chosen during KH1's cast still
  cuts it once the spell is out (the combo's next step).
- "revert this back to before we asked to make any revenge value changes": s179's silent
  multi-hit ticks and s184's engine re-flinch are gone; every hit lands as it did before.
- The tint: Sora's PC model is the remaster's (9 meshes on 4 textures, other sizes than the PS2
  file), so the clothes meshes aren't known; this build colours each mesh plainly in the form
  (0 red .. 8 orange, `tint::PROBE`) to tell them apart from a screenshot.
- "revert the recent aerial dodge change": s183's air control (the stick steering what he has,
  an Aerial Dodge's speed kept under it) is back to KH2's plain steer toward the stick's
  speed; its test `dodge_momentum_kept_with_the_stick` is gone.

### 2026-10-18: Stage 194 (Wisdom's air shots move and fall)
- "in KH2, wisdom forms aerial combat allows you to move in the air while the attack plays,
  and sora also has gravity": an attack whose motion has an allow-movement range steers in the
  air with the stick (air control as in a jump) and ignores its no-gravity ranges (181: 16..40),
  turning with the stick when nothing is targeted. Test `moving_air_attack_steers_and_falls`.
- A public copy of the KH1 mod alone was made (fresh history, `local/export_public.py`), on the PC beside
  this repo (`kh2-combat-public`).

### 2026-10-18: Stage 195 (Wisdom's blue by texture swap)
- "the outfit isnt blue": the PC draws the remastered Sora (its meshes go through `0x1401DB690`,
  never the hooked `0x1401DB7F0`), and his jumpsuit shares one HD texture (`-1c1`) with the
  jacket, zipper and some skin (a helper session's research, `docs/kh1-tint.md`). `tint.rs` now
  swaps the source textures of mesh-table entries 0 and 1 for recoloured copies (red → hue 225°,
  the rest untouched) made with KH1's own texture creator, in the form only. The PKG's remastered
  assets are readable now (`egs::Archive::read_remastered`, `Kh1Install::read_remastered`).

### 2026-10-18: Stage 196 (every KH2 particle program the spells use)
- "wisdom form's fire is lackluster, add all the KH2 vfx": a helper session audited every spell
  package the mod plays (Fire / Blizzard / Thunder / Cure / Reflect, Wisdom's and Sora's) and
  implemented the 20 program types kh2-fx skipped (`docs/kh2-effects.md` §7.5..7.10): the shape
  trails (155 / 157, `kh2-fx/src/tail.rs`: Fire's flame tails, Blizzard's sparkles, Reflect's
  streaks), the loop integrators (210..215), random vectors (63 / 75 / 76), the camera-facing
  and parent matrices, two model draws and vertex births. KH2's heat haze (160, 238) runs but
  draws nothing (it needs the screen behind it). Test `spell_packages_fully_supported` (with
  `KH2_FX_DIR`). The hook now passes the frame's camera before the effects tick
  (`Effects::set_camera`).

### 2026-10-18: Stage 197 (one direction per combo; forms run on MP)
- "sora can sometimes change the direction of his velocity depending on where you hold the
  stick regardless of if hes locked onto an entity or not" / "we do need to change how sora is
  able to change directions when attacking when he isn't locked onto anything", then the rule
  (the user): the combo stays locked in one direction until it ends (idle or a fall); with the
  orange or blue reticle on, the next attack snaps toward the enemy at its start, never mid
  attack. Each swing used to face the stick when nothing was targeted, and the motions' turn
  windows homed mid-swing. `Character::set_combo_direction_lock` (with `set_face_lock`), set by
  the hook. Test `combo_keeps_its_direction`.
- "drive forms consume mp over time, if mp reaches zero it reverts back instead of the current
  timer": 1 MP every 3 s of battle, 6 s out of it; at 0 MP, Revert. Entering needs more than
  the 3 MP it costs.

### 2026-10-18: Stage 198 (no spell held through a finisher; idle ends the combo)
- "during a finisher if a magic spell is inputted, then at the end of the finisher regardless of
  how much time is passed sora will always execute said spell ... there shouldn't be any buffer
  system like that for magic": a spell chosen in a move with no window left for it
  (`Character::magic_later`: a finisher without a next finisher) is refused at once (KH1's
  spells in `combat::command`, Reflect and Wisdom's spells in their waits); one chosen in a swing
  still goes at that swing's combo window.
- "if sora inputs a magic and returns to the idle state his position in the combo string is
  kept ... the intended behavior is for sora to have his combo reset if he returns to idle":
  `COMBO_GRACE` 60 → 2 frames; a KH2 cast played to its end on the ground resets the combo; a KH1
  spell's combo slot is restored at hand-back only when an Attack or a spell cut it or waited
  through it (`takeover::SPELL_CUT`).

### 2026-10-18: Stage 199 (turning mid-swing only toward a reticle)
- "have sora change the direction he's facing mid move only if the condition of an entity
  having a blue or orange reticle on it is met": with the combo's direction locked, the swings'
  turn windows run again when a target is locked (KH1's reticle, either kind, is the lock), and
  stay off without one. Test `combo_keeps_its_direction` extended.

### 2026-10-18: Stage 200 (forms drain yellow MP)
- "for drive forms, instead draining one bar of blue mp we have it drain yellow mp incrementally,
  when the value that stores yellow mp reaches zero then we take away one bar of blue mp": that
  is exactly KH1's payment of a cost under 100 (`docs/kh1-drive.md`): the form pays 1 every 2
  frames of battle (4 out of it); out of MP (KH1's own test fails for 1), Revert.

### 2026-10-18: Stage 201 (hair, hood and keychain move as KH2 animates them)
- "add the animations for soras hair and chain on each kh2 animation instead [of the physics],
  don't affect the KH1 animations": a helper session matched KH1's 14 hair spikes to KH2's 11
  (nearest roots on scaled heads, blended with the second nearest), the hood (KH1 80/81 ← KH2
  72, 82/83 ← 73) and the keychain (KH1 283..291 ← KH2 weapon 3..11, same places in the two
  binds), and `kh1-data/src/secondary.rs` turns them as KH2's motion turns its own (hair and
  hood relative to the head and chest, the chain link by link from KH1's pommel), lengths kept,
  guarded against the head and spine (`docs/kh1-secondary-motion.md`). The KH2 pose the hook
  keeps now carries the Keyblade's bones too. The physics swing runs only for chains KH2 didn't
  pose (the dream weapons and wooden sword). Ini: `physics = off` (all off), `kh2_hair = off`
  (the old swing). KH1's own animations never reach this.

### 2026-10-18: Stage 202 (the user's list: camera, Wisdom, guard)
- Wisdom: "finishing magic ... unable to be in the 2nd finisher slot when having a finishing
  plus": Wisdom's finisher (154) and finishing casts have no finisher range, so nothing could
  follow them. A finisher motion with none now takes the next finisher (swing or spell) in its
  last 6 frames (`finisher_window`). Test `second_finisher_after_a_finisher_without_a_window`.
- "when inputting a 2nd finisher, if you input magic then hit the attack command the attack will
  overide the magic": while a chosen spell waits for its window (KH1's queue, Reflect's,
  Wisdom's), an Attack press doesn't take the window from it (`takeover`).
- "sora should not loose his velocity when doing an air move in wisdom form", "his air attack
  ... freely able to move", "landing animation should preserve his momentum": a moving air
  attack (an allow-movement range) keeps his speed at its start; moving casts keep speed and
  fall and are steered in the air as a jump; with ground inertia the landing slides on instead
  of KH2's 14-frame hold. Test `wisdom_keeps_momentum_in_the_air_and_on_landing`.
- "animations that have new magic do not have their proper elemental hitbox and currently do
  physical damage": Wisdom's Fire / Blizzard / Thunder now land with KH1's own attack data for
  the spell at its level (element, max-MP source, reactions; a helper session's research,
  `docs/kh1-attack-elements.md`); the shots stay physical (KH1 has no known light element).
- "any kh1 magic that doesn't have a kh2 equivalent (gravity, stop) should use the normal
  thunder/cure wisdom form animation but still cast their respected magic": KH1 casts them; in
  the form its slots are drawn with the form's motions, the cast as Thunder's (Gravity) or
  Cure's (Stop, Aero), the air ones in the air (`pose::follow`).
- "Guard currently does not change soras direction to face an entity with a yellow or blue
  reticle": the guard starts facing the locked target. Test `guard_faces_the_locked_target`.
- KH2 camera: "any time that a move or action calls for the camera to be reset (limits,
  inspecting an object) the game reverts the camera back to kh1s camera ... it should call the
  default position of the kh2 camera instead": every KH1 camera reset ends with its follow
  camera starting over from hold (mode 0; event holds and "camera behind Sora", the end of a
  Limit's or summon's cinematic, room loads). A helper session made the KH2 camera catch that
  and start again at its own default behind Sora (facing the target when locked), keeping the
  view through the rest of a scene unless KH1 walks Sora itself (`docs/kh1-camera.md` 9.1, 9.2).
  Room entries now start behind Sora.

### 2026-10-18: Stage 203 (the Magic menu takes a spell only in KH2's windows)
- The user, on Attack taking a chosen spell's window: "in kh2 the game won't let you select a
  magic until the game lets you, keeping you in the menu until you back out with the back
  button". KH1's menu entry run (`0x1402912F0`, hooked in `combat::run_entry`) now returns
  without writing the command record when the entry is a spell (from the Magic list, a party
  target list or a shortcut) and KH2's move can't be cut by magic this frame (`can_cast`):
  no sound, the menu stays where it is, and the next press tries again. So a spell is never
  chosen to wait for a window, and an Attack pressed meanwhile is simply the combo's next
  swing. Drive forms and KH1's own casts keep their paths.

### 2026-10-18: Stage 204 (Drive and Revert on end frames, no attack buffer, air casts hold)
- Drive (the user's list): "you can't transform in the air ... sora inputs a drive in the air
  and should stay frozen in that position": the Drive is taken in the air and Sora is held where
  he is through it (`Character::play_held`: a scripted motion with no gravity and no drift),
  the arrival too, then he falls. "transforming ... should only take 45 frames": Sora's Drive
  motion runs 160 frames (80 at 30 fps); the swap now comes at 39, plus the form's 6-frame
  arrival (53), 45 in all.
- Revert: base Sora's motion 4, which the revert played, is his fall loop (A004), hence "stuck
  in the kh2 free fall animation". The revert now plays nothing: what was playing is cut and he
  is free with his movement carried over (`Character::carry_movement`: speed, fall or rise, in
  the air or not), so on the ground the stick's walk/run/idle takes over without halting him and
  in the air he falls. It works in the air too. Out of MP it happens at the first frame he can.
- Page 2's Drive row reads "Revert" in the form and reverts directly (no list). Drive and
  Revert are taken only on a frame Sora can change (`Character::can_change_form`: free,
  jumping, falling, landing, or an attack / cast in its combo, finisher or reaction-command
  window or last 6 frames): before that the press does nothing (`combat::run_entry`), as spells
  since s203.
- Attack (the user: "there is a buffer system ... We don't want a buffer type system to be in
  the game and only inputs to be accepted when its in a acceptable window"): `character-core`'s
  `buffered_attack` is gone; a press counts only on a frame the combo / finisher / cast window is
  open. An Attack pressed during one of KH1's moves (its spell's cast, a reaction) counts only on
  the frame it is pressed: if it can't cut the move then, it's dropped. Tests
  `an_early_attack_press_is_not_kept`, `drive_in_the_air_holds_and_swaps_carry_movement`.
- Air casts (the user: Reflect in the air "doesn't halt his gravity for a second ... like other
  magic moves do", Wisdom's too): the no-gravity frames of a cast motion hold him again (s174
  had dropped them for air casts so they fell as KH1's do); `air_magic_falls` now only ends a
  cast that reaches the ground in the landing.

### 2026-10-18: Stage 205 (footsteps follow KH2)
- "footsteps currently follow whatever the kh1 animations are doing ... we need footstep sfxs to
  follow what kh2 animations are doing": stage 155 gated `0x140293C70`, which is objects' (crates,
  pushables); Sora's steps come from his KH1 motion's sound events (`0x14029DA4E` ->
  `0x140293290` -> table `0x140529788` -> `0x1402939B0 (actor, kind)`, Steam `0x140295B40`):
  kinds 0..7 the feet (alternated by `+0x39C`), 10 a landing, others walls and objects. Hooked:
  while KH2 has Sora, KH1's are dropped.
- KH2's footstep is frame trigger 2 (param: kind, KH1's own numbering: run 0, walk 4, landing
  10). `kh2-data` loads them per motion and slot (`Rig::footsteps`, battle and field), the clip
  events carry them (`EventKind::Footstep`); the hook plays KH1's floor sound through
  `0x1402939B0` when the rig's walk / run track passes one, and for an action's events (the
  landing).
- Wisdom Form: none but the landing's ("we generally want footsteps disabled for sora in this
  form (with the exception of landing) as he is gliding"; its walk and run have no trigger 2).

### 2026-10-19: Stage 206 (air Reflect's sink, Reflect's guard, Drive revert, Quick Run landing, Wisdom's aura and KH1 spells)
- "revert our changes to base reflect and wisdom reflect, we do want it to fall like kh1" (then,
  asked: it "falls too fast"): s204's hold in the cast's no-gravity frames is undone; the cast
  stops his fall and he sinks at a share of gravity while it plays (`Character::
  air_magic_gravity`, ini `air_cast_fall`, default 0.3, estimated), the fall after it going on
  from the speed reached. Tests `air_cast_sinks_at_its_share_of_gravity`,
  `falling_air_cast_lands_into_the_landing` (now falls).
- "Reflect does not have a block hitbox": the barrier (ATKP 573's shapes) meets the enemies' attack
  objects each frame (`Strikes::barrier_meet`): a free-standing shot it touches gets KH1's clash,
  its half (`0x1402B2A30 (shot, barrier)`, Steam `0x2B4BC0`, the barrier a priority-100 guard
  object): turned back, counted for the counter; an enemy's melee attack gets KH1's clash record
  on its owner (kind 6 / 7, its script's recoil), with KH1's spark. Not tested in game.
- Drive: a form or Revert chosen, KH1's confirm bits are taken out of its battle button state
  until confirm is let go (the user: holding it after a revert opened the Drive list again). The
  revert starts Sora's package elements 15..27 (the Drive motion's 1..13 mirrored; 15 has sound
  115 as 1 has the change's 114; Data, KH2's call not traced), under `form::REVERT_FX`.
- Wisdom's arrival: its light on his body (elements 1..12, the form's colour) stays 90 frames
  whatever he does; its element 100 (the looping aura: sparkles at his feet, trails round him)
  stays for the whole form (s192 had ended both with the 6-frame arrival motion).
- Quick Run ends in KH2's Land (an air slide; the user: no landing after it); landing in a Quick
  Run, an Aerial Dodge or a jump's take-off goes to Land too. Test `quick_run_ends_in_the_landing`.
- KH1-only spells in a form (Gravity, Stop, Aero; drawn with the form's motions since s202): on the
  form's Thunder cast motion's clock (MAGC at the spell's level): Attack / a spell / jump / Square
  cut it only from that motion's reaction window, and it ends at that motion's length once the
  spell is out; on the ground he moves as the form's casts do (the stick in the motion's
  allow-movement range with the form's inertia, braking after), through KH1's mover
  (`takeover::FormCast`).
- KH1's leap off a ladder or a pole (state 16) is handed to KH2's fall once it drops (the user: KH1's
  fall; the log: states 15, 16, then 2 only ~35 above the floor).
- "sora is able to cancel his taking damage animations into attack": a damage reaction takes no
  Attack before it ends (a jump or the action button still cut it from its reaction window).
  Test `sora_flinches_and_is_pushed_back_by_a_small_hit` updated.
- Open: Wisdom's elemental hits still showed the physical spark in s205 (s202's spell data: its
  `+0x28` hit effect is looked up in Sora's effect group, `docs/kh1-attack-elements.md` §4).

### 2026-10-19: Stage 207 (the user's list: ledges, KH1's air states, lock-on, Wisdom's shoes, the Keyblade's vanish)
- "When sora lets go of a ledge he instantly regrabs the ledge": after KH1 lets go of him in the air
  (a ledge, a ladder, a pole) KH1's grabs aren't tried for 20 frames of KH2's fall (`NO_GRAB`), as
  KH1 skips them for 15 frames after jumping off something.
- "KH2 soras falling animation is unable to grab onto ladders/vines": the grabs are tried out of an
  Aerial Dodge or a glide too, and with the stick let go the probe move presses along his drift (or
  facing), so KH1's climb check has a wall face to read.
- KH1's jump or fall with him free (state 1 / 2, KH1's can-act test passing, not a spell's air
  cast): KH2 takes him at once (two frames in), rising or falling (`Character::carry_into_air` now
  carries a rise), its Aerial Dodge ready (`allow_air_dodge`); KH1's jump and Square are KH2's
  meanwhile, so KH1's glide and Superglide don't come out of it (the user's fail-safe). All through
  one of KH1's commands (a spell, an item) KH1's jump and Square bits are KH2's too: on the frame it
  lets him act again KH1 read them as its own jump, high jump or dodge roll (the user).
- Lock-on: locked onto a target, the combo rows are measured against that target only (the user:
  one ahead locked, another beside him: the base finisher, lunging at the one beside him).
- Wisdom's blue takes in the shoes: mesh-table entry 3 (`-1c3`, shoes and the yellow pouches), its
  yellow (hue 48° ± 16) to blue (`tint::FROM`). The pouches go blue with them (same texture).
- The Keyblade's vanish: as KH2 puts it away, element 0 of the Keyblade's own package
  (`obj/W_EX010.a.us`: a flash that swells and fades over 13 ticks with a turning glint) starts on
  the blade (half way from the hand to the tip). Inferred: KH2's own call for it isn't traced.
- Open: after a boss falls, KH1 holding Sora with its own animations before the scene (needs a log).

### 2026-10-19: Stage 208 (the user's lists: KH1 takeover frames, walking, Wisdom's KH1 spells, Reflect, hits)
- Built on s207 (claude-6d's s206/s207 from the GitHub line, applied onto s205).
- Walking: the stick's direction is now KH1's move vector (`0x142D5F630`, written by its think
  `0x1402A4090` -> `0x14028A650`: the stick through the camera), not Sora's `+0x20`, which
  KH1's walk turns toward the stick slowly ("only keeps him moving at the direction hes facing
  prior"). The same for the form casts of KH1 spells (s206) ("sora does not change his movement
  direction to the analogs sticks direction"). A log line when the two differ.
- Wisdom's KH1-only spells: the form's cast clock ends at its motion's length (Thunder 62: 25
  frames at 30 fps = 50) whether or not a shot was counted ("kh1 stays in the animation for
  another 17 frames"), on frame 50; his slide starts from KH2's speed going in and is handed to
  KH2 at the end (`Character::carry_on_ground`; "we don't want sora to have his momentum
  halted").
- KH1 takeover frames (`takeover::kh1_cut_frame`): grounded KH1 Fire from frame 49, taken only
  with an Attack or a Drive chosen then ("only if conditions are met"); an item from frame 51,
  taken whatever is pressed ("on frame 52 ... they will all be kh1s animations"). Items are told
  by the list they were picked from (`combat::run_entry`: KH1's Items list `0x140529670` or a
  party list under it). A Drive chosen during KH1's move goes through `take_chosen_command`
  to `form::choose` (`accept_now` / `kh1_cut_window`), KH2 takes him (`DRIVE_CUT`) and changes.
- Air KH1 spells: KH2 lands him on the frame he would touch down (`touchdown`: falling with
  less than 1.5 frames' drop to the floor, put down through KH1's mover), not a frame after
  KH1's landing (its roll or jump could come out there).
- Air Reflect (base and Wisdom): Final Form's physics (`Character::hold_air_cast`, P_EX100_TRIF
  D373: stopped, no gravity frames 0..54, no drift, then a plain fall), their own animations.
  Test `air_reflect_holds_as_final_forms`.
- Revert: the command menu's cursor back on Attack (root `+0xC`).
- Wisdom Fire / Blizzard / Thunder hits: the attack's effect object (`+8`) is KH1's magic
  effects (`0x142EFBE90`, as KH1's own spell handlers `0x14039FD60` and others set it), not the
  Keyblade's (the row's attack takes the weapon's, `0x1402B10E0`), so the hit effect
  (`0x1402B2190`) is the spells' (the user: "still the physical keyblade effect"). Unconfirmed.

### 2026-10-19: Stage 209 (the user's s208 report)
- Walking: under KH1's own dead zone (a slow walk, KH1 still shows idle) its move vector is
  empty, and the fallback was his stale `+0x20`. Now the raw stick turned through KH1's camera
  as its think does (`0x14028A650`: rotate by minus `0x14299C504`, mirror x, rotate by
  `0x14299CB94`; not in camera modes 2 / 0xB), its 8 sign conventions scored against KH1's own
  vector on frames that have one (`takeover::stick_through_camera`). KH1 refreshes its vector only
  while Sora can act, so during its casts it was stale too (Wisdom's KH1-spell slide kept the old
  direction): it counts only then.
- Wisdom's KH1 spells: the form's cast clock ends the air ones too (KH2's fall from there).
- KH1's menu takes nothing while Sora can't act (`0x1402912F0`, its `0x1402A0D30` test), which is
  why Attack only came out at Fire's end and the Drive list was greyed: in KH1's cut window
  (`kh1_cut_window`) `combat::run_entry` takes Attack (`ATTACK_AFTER`) and a Drive form
  (`drive::chosen`, the menu chain back to its top) itself; `form::usable` counts the window.
- Items: command type 3 (the log's "command 0x3 (02000803)"), not the list they came from (the
  item end frame never fired).
- KH2's first frames after KH1's move don't carry KH1's push speed (`+0x120`, KH1's own walk /
  run speed then, 708/s in the log): the teleport forward at the next attack. Those frames are
  logged.
- Undone: s208's spell hit effect object. `0x142EFBE90` is the current object of KH1's script
  command dispatcher (`0x1403975E0` stores it before each handler), not a lasting magic effect
  set; KH1's element hit sparks belong to its spell objects, which exist only during its own
  casts. Still open.

### 2026-10-19: Stage 210 (KH1's leftover motion after a cut)
- The user: past frame 50 of a form's KH1 spell, KH1 "still thinks its in the aero animation
  (0x3A)" until the stick moves, and an Attack waits 21-25 frames. The s209 log: the cut handed
  him to KH2, and the next frame "KH1's own action, slot 55" took him back (KH1's cast motion
  plays on after our cut). Now a cut (`cancel`, or the air cast's landing) sets `CUT_LEFT`:
  until KH1's slot is a plain one (0..7) or KH1 takes him for something new, the leftover isn't
  KH1's own action. KH1 greys its menu while its cast plays, so Attack is taken in
  `combat::run_entry` as KH2's press meanwhile (`takeover::cut_left`).

### 2026-10-19: Stage 211 (Valor Form)
- The user: "i want to try to add valor form, and add the slot in menu for valor form, and
  automatically put a keyblade that isnt equipped into the slot. If no keyblade is eligible to be
  in that slot, dont let players go into valor form".
- `form.rs` takes several forms (`form::Kind`: Valor, Wisdom, in KH2's order; `drive::FORMS`
  gets Valor first, record 0x7F, word 0x1FD; Wisdom keeps 0x7E / 0x1FE, its argument now 0x11).
  Valor: `P_EX100_BTLF` (moveset 2: its swings with both Keyblades' hitboxes, the rig's two
  weapons), High Jump MAX (as Wisdom's Quick Run), no magic (spells refused with KH1's "can't"),
  its motions' effect elements not played (they're its own archive's, not loaded yet), Sora's
  footsteps. Wisdom-only paths (`formmagic`, the KH1-spell casts on Wisdom's motions, the tint,
  the footsteps rule) test `form::in_wisdom`.
- Valor's Keyblade (`valor.rs`, `equip.rs` from a helper session: `docs/kh1-equipment.md`): a
  Keyblade Sora owns with a copy not equipped; the save's `+0x488` keeps it while eligible, else
  the one with the most Strength; none: Valor greyed and refused.
- Stage 211b: Valor's effects: its archive (`P_EX100_BTLF.a.us`, PAX "p_ex") is effect package
  `kh2fx::VALOR` (after Wisdom's spells'); its motions' frame trigger 1 starts those elements on
  Sora (`kh2fx::form_event`), the ones flagged to end with their motion ended when it changes.
  No element sounds for them yet.

### 2026-10-19: Stage 212 (Valor's row on the Equipment page; Valor's effects)
- A helper session added `equipmenu.rs` (`docs/kh1-equipment.md`, "Implemented"): Sora's slots
  window gets a "V: <Keyblade>" row under his weapon row (rows + 1 through a wrapper on the
  window's update pointer, a shadow of his record behind `r12`), confirm on it opens KH1's
  weapon list filtered to the free Keyblades, the pick stored at `save + 0x488` with Sora's own
  weapon put back; help line and accessory rows mapped one down. All "Equipment:" in the log.
- Valor's effects (stage 211b).

### 2026-10-19: Stage 213 (Valor's left-hand Keyblade)
- A helper session added `leftblade.rs` (`docs/kh1-weapon-second.md`, its section 7 the checks):
  in Valor the Valor Keyblade's `.wpn` is loaded with KH1's loader into our own buffer (KH1's
  `.wpn` release handler `0x1404DAF48` wrapped to skip it), set up as a model, and drawn by a
  second call of the model queue `0x1401D1F10` with Sora's body meshes hidden and the left-hand
  matrices in bones 280..292 for that call only (from KH2's own left Keyblade, the rig's second
  weapon, calibrated on the right hand). ini `valor_left_blade` (default on; off = nothing
  installed). All "left Keyblade:" in the log.

### 2026-10-19: Stage 214 (Valor: Keyblades kept out, Upper Slash)
- "dont let soras keyblades disappear in valor form": in a Drive Form the rig is always in its
  battle stance (`anim.field` false), so the Keyblade isn't put away out of battle (KH2's forms
  hold theirs); the left one (s213) draws only while the right one is out.
- "make sure that soras square launch attack works in valor": KH2's Valor table has none of
  Sora's Square moves (its kind-0 rows are 11, 12, 38, 40: guard and finishers). Sora's Upper
  Slash (row 37, motion 167, ability 95) is copied into Valor's data at load
  (`pose::port_moves`: the row, the clip, the body's and the right Keyblade's animation), usable
  while Sora has Upper Slash equipped.

### 2026-10-19: Stage 215 (sounds: Reflect for everyone, Valor's own)
- "the reflect sound FX arent working for some people": Reflect's (and the forms') sounds were
  loaded only with `kh2_sounds` on, and one unreadable file (Sora's `P_EX100.a.us`, a sound bank
  that didn't parse) dropped every KH2 sound. Now Reflect's and the forms' load whatever
  `kh2_sounds` says (it decides Sora's moves only), and a missing package or bank costs only its
  own sounds (logged).
- "make sure also that valor form has its proper sound FX": Valor's effect elements' sounds (its
  archive's PAX, package `kh2fx::VALOR`, played as they start), its Keyblades' swing sounds and
  voices (its clips' frame triggers 8 / 14) load and play in the form.

### 2026-10-19: Stage 216 (Valor's Square, the left Keyblade's load)
- "using sora's square move in valor doesnt launch him still, he also should be able to use it
  while walking": Valor's guard row (11) is an attack (motion 181, a rising slash with both
  Keyblades, jump 90); the log showed it run as a guard. A guard row whose clip has hitboxes and
  no guard window is now an attack: Square on the ground, standing or moving, starts it as a
  forced row (its launch from the row's jump). Test `guard_row_attack_launches_from_square`.
- "the second keyblade isnt there": the log, "item 0x5b's model not used: header 0x80 0xa800
  0x1f700 (size 0x1f690)": the `.wpn`'s `+0xC` runs past the bytes loaded (end padding), and
  `leftblade::set_up` refused it. Only `+4` / `+8` are checked now, as KH1's own handler reads.

### 2026-10-19: Stage 217 (Valor's Keyblade swap in the form, sound diagnostics)
- "when switching the second keyblade out while in valor form it doesnt swap properly": the
  left Keyblade was loaded only at the form's start. While he is Valor the slot is read every 10
  frames (`form::after_step`) and `leftblade::show` gets it each time (no-op when it's loaded;
  asks again while another load is under way).
- "the sounds are still not there": the s216 log has Valor's sounds loaded (package 17's
  elements, its swings 5000xx) and the volume offsets check out against the Steam exe (KH1's own
  reads of `0x14012AC50`'s settings, all 4 bytes later). `kh2sound` now logs its first 40 plays
  ("KH2 sound played: ..."), sounds asked for but not loaded, a muted volume, and Windows
  refusing the sound device, which `play` dropped silently.
- The left Keyblade's trail: a helper session is on it.

### 2026-10-19: Stage 218 (the left Keyblade's trail)
- "the secondary keyblade doesnt have its trail": a helper session gave it KH1's own swing trail
  (`leftblade.rs`, `docs/kh1-weapon-second.md` section 8): the trail id KH1 is asked for Sora
  (`0x1402C0160`, hooked) on an owner of ours (a copy of Sora's actor and instance with the
  left-hand weapon bones), made as the left weapon's own trail windows open (read from
  `W_EX010_BTLF_L`), faded as they close. "left Keyblade: trail ..." in the log.

### 2026-10-19: Stage 219 (KH1's motion after its air spell's touchdown)
- The user's videos (a memory watch on Sora's `+0x164` motion request, `+0x168`, `+0x16C`
  time): landing from a KH1 air spell, KH2 showed its jump pose, KH1's motion stayed the cast's
  (0x84, air Blizzard) for 6 frames, then KH1's own landing (0x07) blended in. Two causes:
  `AIR_START` read KH1's height above the floor (`+0x3D4`) right after we put him on it, still
  its last frame's ~40, so KH2 started him in the air; and KH1's cast motion went on with its
  own logic. Now the touchdown doesn't start KH2 in the air, and on every cut of a KH1 move
  (`cancel`, the air cast's landing) KH1's motion is set to its idle (`0x14029D7F0 (actor, 0,
  0)`), logged.

### 2026-10-19: Stage 220 (Valor's second Keyblade: trail, menu, stats; forms' MP; Sephiroth)
- The left trail followed the right blade: KH1 takes a point along a bone from its parent's
  world matrix and the bone's local offset (`0x1401D41C0`), and the grip's parent is the right
  hand. The trail owner's matrices are now all of Sora's moved by the right-grip-to-left-grip
  turn. Its look is the left Keyblade's own: the `.wpn`'s effect section (`+4..+8`) is set up as
  KH1's handler does (`0x1401DDF20`, released with `0x1401DED70`) and used for the trail (the
  right one's when it has no such trail).
- Equipment: KH1's live preview runs during the Valor pick (the hovered Keyblade shown, put back
  by KH1's own restore on close).
- Valor adds its Keyblade's Strength and max MP to Sora's each step (from KH1's own figures,
  `0x1402A2FA0`), taken off when it ends (`valorstats.rs`).
- Valor's fall and landing clips are base Sora's no-Keyblade ones (identical in both msets): its
  idle stands in for the landing (`RigAnim::stand_in`).
- Forms drain one unit every 3 frames of battle (was 2), and each melee hit in a form charges MP
  with KH1's own charge (`0x1402A2950`): the hit's units (attack data `+0x2C`) × his bars, as a
  hit is to a point outside a form.
- Sephiroth's pillar killed through Leaf Bracer / Second Chance: a lasting KH1 attack re-arms its
  hit on an interval and KH2's reaction kept Sora in it. No hit lands while he is knocked down,
  nor the same attacker's within 20 frames while he flinches (`hurt.rs`, `docs/kh1-rehit.md`).

### 2026-10-19: Stage 221 (an air spell's touchdown, again)
- The user: s219's landing fix changed nothing. KH1's set-motion (`0x14029D7F0`) only queues a
  request (`+0x178` pending, `+0x17C` the motion); our state change to the ground one ran after
  it, and leaving KH1's air state queues its own landing (0x07) in its place. Now the state is
  set first and KH1's idle queued last. And the touchdown put him on the floor while KH1's height
  above it (`+0x3D4`) still read its last frame's ~40, so KH2's first step started him in the
  air (its jump pose): set to 0 there. KH1's motion fields are logged for 12 steps after a cut.

### 2026-10-19: Stage 222 (the left trail's place)
- The user's video (s221): the left trail, in the Valor Keyblade's own look, was drawn well off
  the blade. Its owner's matrices were made from our pose (KH1 model space × our world) at draw
  time; now each KH2 step from Sora's own world matrices as KH1 last drew him (what KH1's right
  trail reads), each moved by the right-grip-to-left-grip turn of the same frame. A live left
  trail's position, the left grip and its last bone are logged for its first steps.
- KH1's air spells back to s207's handling (the user: "revert the changes made to aerial magic to
  before s207 ... its completely fucked atm"): KH2 takes him when KH1 has landed him (state 0 on
  the floor), with KH2's landing; out are s208's touchdown a frame early (he was moved onto the
  floor), s209's form casts ended in the air, s219's / s221's KH1 idle request, cut flag and
  height reset on the landing. KH2's air Reflect (Final Form's hold) stays.
- Valor refuses KH1's specials (Sonic Blade, Strike Raid, ...): every command of Sora's KH1 runs
  as a move other than Attack, magic, items and its two air-allowed types, logged.

### 2026-10-19: Stage 223 (the left trail's turn)
- s222's log: the left trail's effect sat on the left grip (a frame behind), yet the video showed
  its ribbon well off the blade in a steady direction. KH1 makes an effect's angles from its
  matrix's rows, normalized (`0x1402A6D60`: atan2 / asin), so a mirrored or sheared grip matrix
  reads as a wrong turn. The owner is now a rigid copy of the right hand's Keyblade (every bone
  × one turn) whose turn is the left grip's made proper (orthonormal, the right grip's scale;
  mirrored, the axis least along the blade turned round), filled at draw time right after the
  left Keyblade is drawn. Both grips' determinants are logged.
- Choosing Valor's Keyblade, KH1's preview puts the hovered one in Sora's weapon byte, so the
  stats shown were his with that Keyblade alone (the user: Strength "capped at 70"): his own
  weapon's Strength and MP are added then (`equipmenu::valor_preview`, `valorstats::add`).
- Wall clinks on again by default (`wall_clink`, the user); both inis set on.

### 2026-10-19: Stage 224 (forms' MP from hits taken out)
- The user: KH1's own MP charge on a hit already reached Sora in forms ("at level 100 it was hard
  to notice"); s220's extra charge per melee hit in a form is taken out. The slower drain stays.

### 2026-10-19: Stage 225 (air Reflect falls as KH1's spells again)
- The user: "revert the changes to reflects physics so that it falls like kh1 magic ... make sure
  it keeps the other changes to reflect just the physics changes". s208's Final Form hold
  (`hold_air_cast`, 54 frames without gravity) is no longer asked for: the air cast sinks at
  `air_cast_fall`'s share of gravity (s206) and lands into KH2's landing (s177). Everything else
  of Reflect's (barrier, guard, counter, sounds, menu, combo) is as it was.

### 2026-10-19: Stage 226 (the user's Reflect / Explosion / air magic list; dream weapons)
- Air casts (Reflect): his fall stops as the cast starts, then full gravity from rest
  (`air_cast_fall` default 1.0; test `air_cast_stops_the_fall_one_frame_then_falls`).
- No spell in KH2's landing (`cast_window`: `Land` no longer takes one; test
  `no_cast_during_the_landing`).
- Reflect (helper, `refl-fix`): one counter at the barrier's end whatever Sora does, all hits
  counted; melee attacks meeting the barrier are counted, not clashed (that recoil looked like a
  counter per hit); its element 11 plays where each attack meets the barrier (sound 139, element
  11's own 174 is empty on PC). Sora is untouchable as in KH1's dodge roll (actor flag 0x40, which
  the hit pass skips: no record, no hit effect, no grab) through KH2's invulnerable frames (Dodge
  Roll, Quick Run, Aerial Dodge, Aerial Recovery) and Reflect (`docs/kh1-intangible.md`).
- Explosion's last blast (ATKP 912: up 15, sideways 30) launches as KH2's default blow's height
  and twice its distance (`combat::FARTHER`).
- KH1's air spells: KH2 lands him on KH1's ground state alone (its height above the floor could
  still be the last frame's, and that frame went to KH1's landing and its jump), only once KH1
  had him in the air for it; KH1 doesn't see jump or Square through the air cast (`probe::pad`).
  KH2's sideways speed is carried into KH1's air state (`+0x3C0`) as the air command starts.
- Dream weapons and the wooden sword (`0x52..=0x55`; a tester: the Dream Sword stretched as a
  movement began, the Dream Shield held otherwise in battle than in cutscenes): every weapon bone
  held rigidly as KH1 holds it from his hand (`pose::dream_hold`, noted from KH1's own pose), also
  through the blend back to KH1's animation.

### 2026-10-19: Stage 227 (riding up a target in the air; Explosion higher)
- The user: locked on in the air, attack after attack rode up the enemy's body until Sora was
  above it and his swings missed. A rising frame took the height KH1's mover left him at, and
  pushed out of an actor's body it lifted him; now the mover may cut a rise short (a ceiling),
  never add to it (logged).
- Explosion's last blast launches a third higher than KH2's default blow (up 32, sideways 60).

### 2026-10-19: Stage 228 (air Reflect: a brief hover, then the fall)
- The user, asked which of the falls tried was closest to KH1's air magic: "brief hover, then
  fall" (s204's). Air casts hold him through their clips' no-gravity frames again, then fall
  (`air_cast_fall`'s share of gravity, default full); a cast reaching the ground still goes into
  KH2's landing. Tests `air_cast_hovers_then_falls`, `air_cast_sinks_at_its_share_of_gravity`
  (after the hold), `falling_air_cast_lands_into_the_landing`.

### 2026-10-19: Stage 229 (Reflect's physics back to s177's, one pop; KH1 footsteps; Valor's landing; Wisdom's KH1 spells cut; Aerial Dodge's sound)
- Air casts fall through their no-gravity frames from rest again (s177's; the user: "roll back
  the physics of reflect to version s177"), into KH2's landing.
- Reflect pops once per cast (helper, `refl-once`): the script's counter attack 574 stands
  frames 30..57 and re-arms every 10/8/6 frames, so it hit each foe 3..5 times before the burst;
  now only the burst (667/668/669, tier from all hits counted) strikes, once per foe at the
  cast's end; 574's effect still plays. Test `reflect_pops_once_for_two_hits`.
- Footsteps: KH1's own only (the step sound unhooked; KH2's footsteps from s205 off).
- Valor: KH2's landing plays again (s220's idle stand-in taken out).
- Wisdom: a KH1-only spell (Gravity, Stop, Aero) is cut by an Attack from its cast motion's input
  frame (Thunder's, `AllowReactionCommand`), whether or not KH1 counted its shot yet.
- Aerial Dodge (motion 202) in `kh2_sounds`'s default list: its sound 999019 at frame 1.

### 2026-10-19: Stage 230 (Drive heals; no specials in Valor; the left Keyblade steady)
- A Drive restores Sora's HP to full as it starts (`form::heal_full`, KH1's own HP change
  `0x1402A2790 (actor, amount)`, which keeps its HUD mirror).
- Valor: KH1's specials still came out (the user: Strike Raid, Ragnarok): the command step runs
  command type 99 as attack row 0x3E (`0x1402A4760`), and s222 had let 99 and 0x47 through. Now
  Valor refuses every command of Sora's KH1 runs as a move but Attack, magic and items, and its
  attack-row executor refuses any row for a command other than Attack (logged).
- The left Keyblade jittered as Valor fell in place: its bones were kept in KH1's world, so a pose
  a frame old was drawn where he had been. Now kept in model space and placed with the frame's
  own placement, and paired with KH2's left weapon of the very step whose body is drawn (a
  signature of the body's bones).

### 2026-10-19: Stage 231 (Final Form, a secret)
- The user: "lets work on implementing final form ... instead of it being listed in the drive
  menu, lets make it a secret": a button code on the pad (kept out of these notes and stored
  unreadably in `form::CODE`, the user; at most 1.5 s between presses; another watched button
  starts it over) asks for Final Form as the Drive list would (`form::pad`, read in
  `probe::pad`; taken in `form::before_step` when KH2 can change him: free, jumping, falling,
  landing or at a move's end, not in a form). The Drive list is unchanged.
- Final Form: `P_EX100_ULTF` (moveset 5, form id 5; TRIF is Master Form), the same change, cost,
  drain and revert as the others, its growth ability Glide at MAX, its own effect package
  (`kh2fx::FINAL`, after Valor's) and sounds, KH1's specials refused as in Valor.
- Its second Keyblade is always the Kingdom Key (item 0x51; the user: "just default it to
  kingdom key and keep the menu how it is"), with its Strength and MP as Valor's Keyblade's.
- Its Keyblades float where KH2's weapon motions put them (W_EX010_ULTF_R / _L on bones 178 / 86;
  their idle roots some 70 units from the hands, Valor's 5): KH1's weapon bones keep the turn from
  base Sora's held Keyblade root to KH1's grip, put on Final's root now
  (`leftblade::final_float`); the left one follows by the same turn (its reach check 400 there).

### 2026-10-19: Stage 232 (Final Form's code kept secret)
- The user: the code is removed from every readable part (notes, comments, logs); the table in
  `form.rs` is stored encoded. No change in play.

### 2026-10-19: Stage 233 (the user's 19:57 list; Final Form's combos)
- Footsteps: KH2's back (s229 undone).
- Air casts (Reflect) fall at KH1's own gravity after the stop: KH1's air state falls
  0.4847 × 0.5 × t² (`0x1402AD3A0`, its step `0x1423405DC` 1.0 at 60 fps) and at most 20 a frame;
  the controller's air cast takes a gravity share over 1 and its own fastest fall
  (`air_magic_fall_max`); `air_cast_fall` is now a share of KH1's.
- Final Form: a swing meeting a wall with no repelled motion in the moveset (Final has no 41)
  went back to its frame 0, met the wall again, for ever: "repeats the same moves ... soft locks
  you" (wall clinks on since s223). Now only pushed back (test
  `wall_hit_without_a_repelled_motion_doesnt_loop`). Its air combo ended after two hits: an air
  attack with an allow-movement window dropped its no-gravity frames (Wisdom's shots' rule); now
  only one moving all through does.
- Lock-on: an air swing at a target just below (within 150 past its radius) hops no higher than
  reaches it (test `air_swing_hops_less_at_a_target_below`).
- KH1 spells: Attack pressed during any of them is taken (KH1's menu refused it while it held
  him) and cuts the spell once it is out (a form's at its cast motion's input frame), the combo
  going on; pressed too early it is dropped.
- Clanks (helper, `clank`): KH2 swings clash along the drawn KH1 Keyblade (six points, KH1's own
  blade range per weapon, KH1's clash radius), swept between frames, against guards, walls and
  enemy attacks (KH1's attack-against-attack clash); Aerial Dodge carries a KH1 guard object (a
  parry). `docs/kh1-clash.md`.
- Reflect (helper, `refl-voice`): the pop at the barrier's end whatever the hits (the hold for
  hits taken out; the pop strikes with the counter at frame 30, the burst row's damage); a random
  KH2 attack grunt on each cast and voice 25 (its finisher line) on the finisher.
- Looks (helper, `form-colors`): Wisdom's sleeves blue; Valor's sleeves, belt, hood lining and
  shoes red (`tint::Look`, `docs/kh1-tint.md`).

### 2026-10-19: Stage 234 (the user's 21:25 list; command menu)
- Lock-on: s233's lower hop at a target below undone.
- Wisdom: an Attack pressed during a KH1 spell while frozen frames hold the cut is carried
  (`CARRY_MAX` 10 frames) and goes on into KH2's combo; Cure is KH1's Cure on KH2 Cure's motion
  (only Fire/Blizzard/Thunder are KH2's in Wisdom), falling and ending like the other KH1 spells.
- Forms get Aerial Recovery when the moveset has its clip; its guard and invulnerable frames come
  from the clip's windows (KH2's frames 0-16), not a fixed rule.
- Aerial Dodge: no invulnerability, only its Guard window (ATKP 406, a KH1 guard object).
- Guards (helper, `guards`): KH2 Guard windows are KH1 pure guard objects (clash type 4, priority
  100) for Aerial Dodge and Aerial Recovery.
- Valor: Square in a combo window goes to the finisher (rows 12 ground, 38 air,
  `square_finishers`); the left trail runs along the second Keyblade's own blade range; the form
  effects anchor on the Keyblades, sent in Sora's facing (helper, `valor-fx`).
- Final: air casts are KH2's own (no KH1 fall).
- Voices (helper, `kh1-voice`): KH1's swing grunt (group 4, random) on the dodge roll; KH1's
  "Give me strength!" (clip 37, inferred) on a Drive.
- Command menu (helper, `cmd-menu`): the Drive list's title reads DRIVES and its panels are white
  (list draw `0x140277F40` hooked); Items and Summons refuse a confirm Sora can't take and stay
  open, like Magic; Magic is greyed in Valor. `docs/kh1-drive.md`.

### 2026-10-19: Stage 235 (KH1's air spells land into KH2's landing)
- The user: "rewrite kh1s magic animations so that if they're used in the air they can call for
  kh2s landing animation". KH1's state change (`0x1402AF7D0`, Steam `0x2B1960`) is hooked: when
  KH1 itself puts Sora down out of an air spell (state 1 / 2 to 0 while it has him for the cast)
  inside his physics step, KH2 takes him at once, after that step on the same frame, into KH2's
  landing (`TOUCHDOWN`, a second `takeover` that frame). Before, KH2 took him a frame later, so
  KH1's landing frame showed and ran first. A landing KH1 makes outside the physics step is
  logged and left to the next frame's hand-off as before.

### 2026-10-19: Stage 236 (move styles: KH1 or KH2 per kind of move)
- The user: toggles for "base combo KH1 | KH2, aerial base combo KH1 | KH2, magic kh1 | kh2, aerial
  magic kh1 | kh2"; asked: in the Config menu and the ini, Gravity / Stop / Aero as KH1 spells on
  KH2 cast motions, base Sora only. `styles.rs` (ini `ground_combo`, `air_combo`, `magic`,
  `air_magic`; defaults combos KH2, magic KH1, as before). `docs/kh1-move-styles.md`.
- Combos at KH1 (helper, `style-combo`, `stylecombo.rs`): Attack runs KH1's own combo (its rows,
  finishers, abilities), KH1 keeps the presses, KH2 takes him back once KH1 lets him act; the
  style is picked by where he is when Attack is pressed.
- Magic at KH2 (helper, `style-magic`): Fire / Blizzard / Thunder / Cure are KH2's base-form spells
  (Wisdom's path, base Sora's MAGC cast motions, magic sub level × 2 − 2); Gravity / Stop / Aero
  KH1's on base Sora's KH2 cast motions (`BASE_KH1_CAST`); Aerial Magic at KH2 gives air casts
  (Reflect too) KH2's air physics.
- Config menu (helper, `style-menu`): KH1 doesn't scroll its list, so ours is a second page (rows
  8-12: Camera Style, then the four styles) drawn in KH1's rows' places while the cursor is past
  row 7; KH1's rows' panels (`0x1402D5590`) are skipped then. s224's 25 pitch is gone.

### 2026-10-19: Stage 237 (the style rows' names)
- The Config rows read as the user wrote them: "Base Combo", "Aerial Base Combo", "Magic",
  "Aerial Magic" (were "Ground Combo", "Aerial Combo"). The ini keys stay `ground_combo` /
  `air_combo`.

### 2026-10-19: Stage 238 (KH1's air spells call KH2's landing, from scratch)
- The user: "scrap everything related to aerial magic transitioning to kh2 landing animation and
  start fresh ... kh1s aerial magic animation states are hardcoded to transition to kh1's landing
  animation ... code [them] so that they are able to call kh2's animation states and call kh2's
  landing animation". Out: s235's state-change hook and its second takeover in a frame, and
  s207's rule (KH2 took him on the frame after KH1 had landed him, its landing queued).
- Where KH1 hard-wires it: Sora's player step `0x1402A75B0` (Steam `0x2A9740`; from the actor
  kinds' switch at `0x14029593C`), on a frame his state changed (flag `0x400`): state 2 to 0
  (old state `+0x378`) asks for KH1's landing, motion `(+0x3D0) + 7 | 0x1000000`, and
  `0x14029D8A0 (0x40000000)`; to state 2 its fall (6). Hooked: during KH1's air spell that landing
  is not asked for (the old state hidden from the step for the call) and `AIR_SPELL_LANDED` hands
  him to KH2 into its landing (`LAND_NOW`). If the step never saw it, KH1's ground state a frame
  later does the same, logged ("... without its landing seen").

### 2026-10-19: Stage 239 (move styles: Jump and Glide)
- The user: "a setting for kh1 jump or kh2 jump and ... kh1 or kh2 glide". `styles::Kind::Jump` /
  `Glide` (ini `jump_style` / `glide_style`, default kh2; `glide` still picks KH2's glide
  button), Config rows 13 / 14 on the second page.
- Jump at KH1 (helper, `style-air`, `stylejump.rs`): a fresh jump press while KH2 has him free on
  the ground runs KH1's ground jump handler (`0x1402CA3F0`: its jump or high jump), KH2's ground
  speed carried; KH1 keeps him through the jump, grabs and landing; KH2 takes him once KH1 lets
  him act for its own move (Attack with Aerial Base Combo KH2, a fresh Square, a jump press
  falling for KH2's jump-button glide, a command, waiting, then by Aerial Magic).
- Glide at KH1: KH2's glide off; a fresh jump press falling in KH2's air puts him in KH1's air
  state 2 and starts KH1's Superglide (`0x1402B05C0`) or Glide (`0x1402B0530`) by its abilities
  (`0x140282A70 (4 / 3, 0xB)`); KH2 takes him back into its fall or landing when it ends.

### 2026-10-19: Stage 240 (move style: Dodge Roll)
- The user: "add an option for switching between dodge roll". `styles::Kind::DodgeRoll` (ini
  `roll_style`, default kh2), Config row 15 (the second page now holds 8 rows).
- KH1 roll (helper, `style-roll`, `styleroll.rs`): KH1 rolls from its button handler
  `0x1402A59A0` (Square in ground state 0, not in deep water `0x1402BF530`, the Dodge Roll ability
  bit `0x20` of `0x142F5F610`; Guard `0x10`), turning him (`0x140295E20` / `0x140295C80`) and
  starting motion `0xDC` (`0x1402C7CB0`). At KH1: a fresh Square with the stick while KH2 has him
  free on the ground runs that, KH1 keeping him while `0xDC` plays; Square standing still stays
  KH2's guard; no ability or deep water, KH2's. Quick Run stays KH2's roll's. Steam map override
  `0x2F5F610:0x2F5EC10` (data).

### 2026-10-19: Stage 241 (air spells' KH2 landing off; jumps out of KH1's moves)
- The user: "comment out anything related to having kh1 magic animations transition into kh2
  landing animation ... causes more bugs then fixes anything". s238's player-step hook is no
  longer installed and the air spell's hand-off at landing is commented out (`cast_landed`
  false): KH1's air spell lands in KH1's landing, KH2 takes him after it as after KH1's other
  moves. The code stays for later.
- After a KH1 air attack (Aerial Base Combo KH1), the user: holding jump through the landing
  jumped on the first free frame ("buffer a jump, this is unintended"); a direction + jump in
  KH1's landing gave KH2's run, not a jump; KH2's high jump out of it went "way higher than it
  should". A jump out of a KH1 move is now only a fresh press made while KH1 has him on the
  ground (`GROUND_JUMP`; a press in the air, or the button held from it, doesn't count). It is
  KH2's jump on KH2's first frame that takes input, frozen frames between (`PENDING_JUMP`, up to
  30): KH2's own "held" memory was stale (the jump KH1's air attack came out of), so a press in
  the landing read as held. KH2's held state is set from the button at the hand-off
  (`JUMP_HELD_OVER`). For 40 frames after a KH1 move KH1's push doesn't lift him in the air (not
  on a platform), and those frames are logged ("after KH1's move: ...") for the high jump, whose
  cause isn't confirmed.

### 2026-10-19: Stage 242 (KH1 glide on Square, KH1 roll, KH2 glide back, Battle Quotes row)
- The user (KH1 glide): "when you input an aerial dodge glide takes priority can you rewrite
  normal glide so that it activates on a square press". KH1's glide starts on Square held while
  falling (KH2's air, or KH1's jump), once a hold; the jump press in KH1's air is KH2's (Aerial
  Dodge). In KH1's glide, Square held is fed to KH1's control step as its jump held.
- "unable to input a kh1 dodge roll": s240 read KH1's ability bits at `0x142F5F610`; they are at
  `0x142D5F610` (Steam `0x142D5EC10`, gen2's override fixed).
- "unable to input a kh2 glide when glide is set to KH2": KH1's glide style zeroed KH2's glide
  level in Sora's abilities, which are only set again when KH1's menu changes; kept aside now and
  given back.
- The user: "an option to switch between kh1/kh2 battle quotes". `styles::Kind::BattleQuotes`
  (ini `battle_quotes`, default from `kh2_voice`, so KH1's), every form; KH2's motion voices all
  load and play by it (`kh2sound::kh2_voice`). Config rows go 8 to a page now: Battle Quotes is
  on a third page (the help line reads it as the second page's row in its place).

### 2026-10-19: Stage 243 (KH1's own jump out of its air attack's landing)
- The user (s241): after a KH1 air attack, a high jump in its landing still launched him, and with
  the stick tilted KH2's run took the jump; not after KH1 magic. The log: KH1's state 0 (can't
  act), then state 1 "can act" on the floor: KH1 jumped him itself (its air attack's landing reads
  the jump press outside its control step, where it is kept from KH1), KH1's high jump; KH2 then
  took him rising 25 a frame (`air_free`, its rise carried), or on the stick at state 1 into its
  run with no jump. Now KH1's take-off out of one of its commands (state 1 on the floor, not the
  Jump style's) hands him to KH2 at once with KH2's jump (`kh1_took_off`), KH1's air fields reset.

### 2026-10-20: Stage 244 (Config menu crash)
- The user's log (s242): a crash entering the Config menu, executing 0x40 from KH1's window list
  step (Steam `exe+2E3E2B`, a window's function pointer). s242's 17th row: KH1's list takes 16
  at most, a 17th writes past the window's row arrays. Back to 16 rows (KH1's 8, our 8 places);
  our page scrolls when it has more (`SCROLL`): Down on its last place / Up on its first moves
  our rows instead of the cursor (KH1's list masks: up `0x10000010`, down `0x40000040`, set at
  `0x1402ED1E0`). Battle Quotes is the 9th, scrolled in below Dodge Roll.

### 2026-10-20: Stage 245 (the user's s244 list: KH1 roll, glide, jump, Drive in KH1 combos)
- KH1 roll (helper, `roll-decouple`): "soras model decouples". KH1's placement (model matrix
  `+0x260`) lags his position (`+0x10`) over its roll and some frames after (its root
  compensation, `0x14029CF10`, read, not confirmed); our draws sat up to ~300 behind him, then
  snapped. Over KH1's roll and 0.6 s after it the drawn placement is his position
  (`pose::on_actor`, logged), and no stale KH2 pose is drawn over the roll's start.
- KH1 jump / glide (`stylejump`) hand-offs strip KH1's jump and Square (they weren't: KH1 had him in
  states 1 / 2 / 21 without its can-act, so `strip` was off): no KH1 glide on the jump button in
  KH1's jump ("still able to do a normal kh1 glide by pressing the jump button"); in KH1's glide
  Square held is its jump held, so the glide holds ("go into the normal kh1 glide for a second
  then get forced out": KH1 ended it after 8 frames, seeing jump up).
- Drive in KH1's own combo: "unable to select drive (it is blacked out)". The cut window
  (`KH1_WINDOW`) is open all through a KH1 combo for a Drive (KH2 takes him to change); its Attack
  stays KH1's (`combat::run_entry`).

### 2026-10-20: Stage 246 (KH2 Thunder's sounds; KH2 battle quotes on every move)
- "KH2 thunder doesnt have any sound FX": the PC log, "KH2 sound 126: scd: no stream for sound
  205". se000's stream table holds 215 streams (`0x34`); the reader took `0x32` (152), so
  Thunder's (126 / 127 / 128 / 148 on entries 205..208) had none. `kh2_formats::scd` reads the
  larger. (Seen meanwhile: the "entries" table read with `0x30`'s 314 runs into the stream table
  past 152, so Reflect's 138 / 139 / 176 (entries 180..184) play streams 28..32 through it, not
  streams 180..184; left as is, the user has heard them since s178.)
- "when switching to KH2 battle quotes, there are no KH2 battle quotes": only the `kh2_sounds`
  list's moves (ability moves by default) had voices loaded and played. Every move's voices load,
  and play on any move with Battle Quotes at KH2; KH1's swing voice stands down for every move
  then.

### 2026-10-20: Stage 247 (KH1's glide bit read as Square)
- The user (s245 log: Jump KH2, Glide KH1, Dodge Roll KH1): jumps cut short of their full
  height ("multiple inputs are being sent ... unable to get the max height"), straight into KH1's
  glide, "still able to input normal glide on the jump button". `takeover::buttons` reads KH1's
  action bit `0x100000` of its held state as Square, and KH1 sets it too with its jump held in
  the air (its glide): KH2 saw Square mid-rise (the rise cut from 14 to 2 a frame at ~126) and
  the Glide style's start (Square held, falling) fired at the top. With the jump held, only the
  pad's own Square counts now (logged when the bit comes with the jump).

### 2026-10-20: Stage 248 (KH1's jump: its height back, no glide on the held jump)
- The user (Jump KH1): "as soon as i jump i go into a glide. I also dont as high of a jump as i
  should be getting because it cuts off the input to the jump button". s245 kept KH1's jump bits
  from it all through its jump, so its jump read as released at once (a short hop). Now while
  KH1's jump rises (its state 1, his height going up) its jump held is KH1's (held only, not the
  presses); from the top on it isn't, so holding it doesn't glide (the glide, Glide at KH1, is on
  Square). The glide right after the jump in s246 was also s247's Square bit.

### 2026-10-20: Stage 249 (no input buffers or queues)
- The user: "comment out any code that is related to queueing/buffering moves? its currently
  breaking a lot of situations". Commented out / switched off, a press counting only on its own
  frame:
  - s241's jump latch (`GROUND_JUMP`: a press in KH1's move kept for its end) and its forced jump
    on KH2's first frame with input (`PENDING_JUMP`); KH2's held state at the hand-off
    (`JUMP_HELD_OVER`) and KH1's own take-off turned into KH2's (`kh1_took_off`) stay.
  - s234's Attack carried through frozen frames (`CARRY_MAX`): dropped at once.
  - `combat::PENDING_COMMAND` waits: a command KH2's move can't take now is refused (the
    menu's "can't"), not kept for its next window; one chosen in KH1's jump / glide / roll is
    taken only if KH1 lets him act on that frame; a spell during KH1's cast only while it could
    cut it now (`takeover::SPELL_CUT_OPEN`), else refused / dropped; what's stored is dropped
    after 0.2 s.
  - KH2 spells' and Reflect's `WAITING` (up to 5 s for the move's next window): dropped at once.

### 2026-10-20: Stage 250 (the dream weapons' stretch, again)
- A tester (the user's relay, Dive to the Heart, screenshots): "tap move, the split second any
  animations begin, he stretches ... his un-stretched shield appears at the end of the stretched
  mesh", not on jumps; a long stretched blade along the floor. s226's rigid hold (`dream_hold`)
  notes KH1's hold from KH1's pose on any frame KH1 posed him; on an animation's first frame some
  weapon bones were still the last drawn ones, far from the hand, and that hold put the weapon
  there. A hold is noted only with every weapon bone within 250 of the grip (logged otherwise).
  The shield's orientation ("isn't perfectly oriented in his hand") is open: no detail given.

### 2026-10-20: Stage 251 (KH1's jump handling off for now)
- The user: comment off, for now, (1) the Jump style's KH1 jump start (`style_air_start`'s jump
  branch: KH1's ground jump handler, KH2's speed into its air speed, its take-off height), (2)
  the jump button kept from KH1 in its jump / held while rising (s245's strip for
  `stylejump::JUMP`, s248's `kh1_rise`; KH1's glide keeps its strip and Square-as-jump-held),
  (3) s247's filter on KH1's action bit (it reads as Square again, as before). The Jump row at
  KH1 does nothing meanwhile (KH2's jump).

### 2026-10-20: Stage 252 (Gummi Routes skip, our own; Misc Animations)
- The user: "can you put an option in the config to skip the gummi routes?", then "no dont use
  their script, recreate it": KH-1FM-AP-LUA's Instant Gummi (s114-s138) not restored. A helper
  (`gummi-own`, `gummi.rs`, `docs/kh1-gummi.md`) read the executable: the world map's task
  (`0x140219490`) starts a route through `0x14020EF10 (kind, stage)`; a cleared flight
  (`0x14023DA40`) lands through KH1's landing list `0x140213B20 (world)`, `0x140266920 (4)`,
  `0x14020ED90 (landing)`. With `gummi = skip` the route start is hooked to run those landing
  calls for the chosen world (`0x140507C94`) at once; story-started flights (`0x14020EEB0`) are
  left. Config row "Gummi Routes" (Vanilla | Skip), our last. Steam: data overrides
  `0x507C94`→`0x506F94`, `0x50B1B4`→`0x50A4D4`.
- The user: "a setting for misc animations to be KH1 or KH2? Like item throwing animation"
  (helper, `misc-anims`): `styles::Kind::MiscAnims` (ini `misc_anims`, default kh1), its row
  before Battle Quotes. At KH2, KH1's item use is drawn with KH2's motion 9 (P_EX100 A110, picked
  by its shape: the hand overhead at 20-30, a voice at 0; unconfirmed it is KH2's item use);
  KH1 still runs the item. Not mapped: summons, examining, picking up.

### 2026-10-20: Stage 253 (why KH1 keeps him: logged)
- The user: "it reverted me to KH1 Animations right after i defeated the card tower" (Wonderland,
  the PC's s249 log). After the boss's scene the log says nothing of who had Sora: "KH1 has
  Sora (why)" stopped at 400 lines (shared with the status lines) long before. A new reason is
  now always logged (at most 4 a second), and while KH1 keeps him outside its own moves for over
  5 s a status line every 10 s: the reason, KH1's state / slot / flags, the event and script
  values, whether his body was seen, takeover on.

### 2026-10-20: Stage 254 (Misc Animations: standing, walking, running too)
- The user: "misc animations: KH1 — sora doesn't switch to kh1s misc animations when kh1 is
  selected" (only the item use was mapped, none in the log); asked, "idle walk/run plus items".
  At KH1, while KH2 stands / walks / runs him on the ground and KH1's slot is one of its own
  0..7 (KH1's control step picks its idle / walk / run from the stick under KH2 all along), KH1's
  own animation is drawn (`takeover::MISC_KH1`, `pose::wanted`, blended in from our last pose),
  KH1's step sounds play and KH2's footsteps don't; KH2 still moves him. The default is now KH2
  (KH2's walk and run as before; KH2's item motion with it).

### 2026-10-20: Stage 255 (Jump at KH1: KH1's jump animations)
- The user: "turning jump to kh1 doesn't change soras jump animation back to kh1" (s251 took
  KH1's own jump out); asked: KH1 animations only, KH2's physics. With Jump at KH1 (base Sora),
  KH2's Jump / Air / Land are drawn with KH1's own motions, evaluated from KH1 Sora's motion
  table (`xa_ex_0010.mset`, `kh1_data::mset`) on his skeleton (`pose::kh1_air_pose`): 12 the
  take-off then 13 the rise, 10 the fall (looping), 11 the landing (held at its end); timed from
  when KH2's state got there (`takeover::KH1_AIR_SLOT`). KH2's Keyblade turn and wrist aren't
  applied over them (`NATIVE_KH1`). Slots from the play logs (s16x), not checked frame by frame.

### 2026-10-20: Stage 256 (KH1's own jump back on)
- The user: "can you actually go with the number 2 option" (asked at s255: turn KH1's jump back
  on). s251 undone: the Jump style's KH1 jump start (KH1's ground jump handler, KH2's speed and
  the take-off height), its jump held KH1's while rising (s248) and kept from it from the top,
  KH1's jump / Square kept from it in its jump (s245), and s247's filter on KH1's action bit.
  s255's KH1 animations over KH2's jump stay for KH2's own falls with Jump at KH1 (after a KH2
  move in the air).

### 2026-10-20: Stage 257 (KH2's player model)
- The user: "an option in config to switch between sora's player model And his KH2 Player
  Model"; asked: KH2's model always (KH1's animation carried onto it), the forms' own KH2 models,
  KH2's Keyblades. Helper (`kh2-model`, `docs/kh1-kh2-model.md`): `kh2_formats::mdlx` (meshes,
  skeleton, textures replayed through the PS2 GS memory layout), `kh2_anim::skin`,
  `kh1_data::inverse` (KH1's pose onto KH2's skeleton), `win/model.rs` (loads P_EX100 and the
  forms' models and the matching KH2 Keyblades from the user's KH2 install, hides KH1's Sora by
  masking his body meshes in the model queue, draws the KH2 model in the effect renderer before
  the effects, with depth and alpha cut-out). Config row "Player Model" (KH1 | KH2), ini
  `player_model`, default kh1. No lighting yet; KH1's Keyblade while KH1 animates him; nothing
  seen in game. The model-queue hook is always installed now (`valor_left_blade` only gates the
  second Keyblade).

### 2026-10-20: Stage 258 (Valor pick's Strength, again)
- The user: "the strength cap is at 70 again" (asked: the Valor Keyblade pick). KH1's preview
  puts a Keyblade in Sora's weapon byte a hover behind, and at times still his own; s223 took
  the byte for the hovered one and added nothing when it was his own, so the stats shown were
  his with his own Keyblade alone. Now: KH1's figure minus the byte's Keyblade, plus his own and
  the one under the cursor (`equipmenu::valor_preview` returns all three, the hovered row from
  the list).

### 2026-10-20: Stage 259 (Player Model off for now)
- The user: the KH2 player model "comment it out for now" (s257's bugs being fixed meanwhile).
  Its Config row is commented out (`config::ROW_KINDS` maps rows to styles explicitly now) and
  `model::on` is false whatever the ini says; the code stays.

### 2026-10-20: Stage 260 (KH2 player model fixes, still off)
- The user's s257 list ("doesnt show up in pause menu ... flickers pretty consistently ...
  doesnt have any VFX for his combos ... KH1 sora animations is super contorted"), helper
  (`kh2-model-2`, `docs/kh1-kh2-model.md`): the pause menu's own Sora copy drawn as KH2's model
  (its KH1 pose carried over, KH1's copy masked, KH1's menu camera); the flicker (the log:
  hidden 18000 times, drawn 9126): the draw now prepared when KH1's Sora is held back and drawn
  on every world pass that hid him, never hidden without it, locks no longer skipped, KH1's
  interpolated-camera frame replay (`0x140102740`, hooked, counted) shows KH1's model while it
  runs, a 20 s counter line; the combo effects: KH1's Keyblade stays queued (shrunk into the
  grip for its draw) so its trail and effects carry on; the contortion: `kh1_data::inverse`
  rebuilt on the forward retarget's body parts (limb directions and knee / elbow roll from KH1's
  joint positions, torso turns blended, KH2's own rest for hair / face / hood / cloth). The
  option stays commented out (s259) until the user turns it back on.

### 2026-10-20: Stage 261 (testing branch: shared states)
- Testing branch only. The user: "lets rewrite how kh2 injects it animations into the game ...
  The goal is to cleanly separate animation handling from the unique logic and physics each
  version of Sora needs". Also "comment the jump codes I just asked you to restore": s251's
  comment-off re-applied (KH1's jump start under KH2, the jump-button strip, s247's filter).
- Every move style is KH1's by default (`styles`, `settings`; an ini that already has a key
  keeps it).
- `shared.rs` (`docs/shared-states.md`, ini `shared = on`): for base Sora, KH1's own state
  machine owns him. Idle, walk, run, item use, jump, high jump, fall and landing run in KH1's
  states:
  - at KH1, unmodded;
  - at KH2, KH1's slots are drawn with KH2's motions (`shared::kh2_look`, `pose::follow`) and
    moved by KH2's numbers (`shared::physics`).

  The physics wraps KH1's state table (`0x142DB4240`): the ground step's displacement is
  scaled to KH2's walk or run speed. KH1's takeoff and its state 2 step follow KH2's jump or
  High Jump arc, gravity, release hop, fall cap and air steering, with KH1's own arc fields
  kept in step.
- KH2's controller only borrows him for KH2's moves (`takeover::shared_frame`):
  - Aerial Dodge, as a double jump with Jump at KH1 too;
  - KH2's glide;
  - Square at Dodge Roll KH2;
  - Attack at a combo style KH2;
  - KH2 spells, Reflect, Drives.

  It gives him back to KH1's ground or fall state when the move is over
  (`shared::give_back`). A ground jump out of a KH2 move is KH1's own.
- Removed: KH1's idle / walk / run and jump drawn back over KH2's (`MISC_KH1`,
  `KH1_AIR_SLOT`, `pose::kh1_air_pose`).
- RE (helper, static): KH1's jump is a 4-frame crouch in state 1 (`0x1402ADC10`), then the arc
  in state 2's step `0x1402AD510` (the same sin-sin rise as KH2's, then `g/2 t²`, capped at 20);
  heights from btltbl (190 / 290); walk 2.0 / run 8.0 in the stats' `+4` / `+8` (stick past
  0.8); landing by collision (`+0x2D0` bit 0).
- Not yet: Base / Aerial Combo, Magic and Dodge Roll at KH2 are KH2 borrows, not KH1's states
  with KH2's motions. Nothing tested in game.

### 2026-10-20: Stage 284 (testing branch: shared combo and glide, KH2 abilities on KH1's moves, split ability ids, 30 fps, KH2's camera in KH1's)
- The user's testing list (s261, 30 fps Steam log): shared animations for base attack 2, the
  finishers and glide; KH2's dodge roll unusable; KH1's roll into Quick Run; KH2's combo,
  Square and general modifiers with the combo styles at KH1; KH2 in slow motion at 30 fps; KH1's
  camera calls taking the camera; split KH1's and KH2's ability ids; docs.
- **Shared ground combo and glide** (`shared.rs`, `pose.rs`): KH1's combo rows and glide state at
  either style, drawn with KH2's swings (98 -> 151, 99 -> 152, 101 -> 154) and glides (113..116)
  at KH2, with KH2's travel and glide numbers. KH2's dodge roll: s261's log had Dodge Roll, Guard
  and Quick Run unequipped (nothing for KH2's Square); Square now borrows only with one of them.
  KH1's roll held into KH2's Quick Run (Quick Run equipped).
- **KH2's abilities on KH1's moves** (`combomods.rs`): KH2 borrows him where it would use an
  ability move (openers, steps, finishers, Square moves, Negative Combo / Finishing Plus,
  Aerial Recovery, Retaliating Slash), KH1's own rows elsewhere.
- **Ability ids split** (`growth.rs`, `menu.rs`, `docs/abilities.md`): KH1's 1..65 vanilla, KH2's
  moves on 78..87 (66..77 as before); an owner table; old saves moved once (mark at save
  `+0x48A`).
- **30 fps** (`takeover::steps`): KH2's side runs whole 60 Hz steps a game frame.
- **Camera** (`camera.rs`, `docs/kh1-kh2-camera.md`): KH2's camera as KH1's own modes; KH1's ledge
  / climb cameras, its lock-on blends and its "held still" scene test no longer take it;
  `camera_calls = kh2` (ini).
