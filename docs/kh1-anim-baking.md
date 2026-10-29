# Baking the rest of Sora's base animations into KH1's slots (research, t480 round)

Thread: `testers/baking-the-rest-of-the-base-animations/`. Related: `kh1-anim-slots.md` (how slots,
requests and the bake work; §1.3 the animation step, §1.6 the pose pass, §3.1 tier B, §8 walk and
run), `animations.md`, `kh1-secondary-motion.md`. The skeleton and the coat are the "Missing sleeves
animation" thread's and are not covered here.

All addresses are **Steam** (the Steam Ghidra decomp, image base 0x140000000). Where an older doc or
our code gives an Epic address, the Steam one here was taken from `steam_map.in` or from the decomp.
No decompiled code here: plain-language notes only.

## 1. What is baked today, and what is still drawn over KH1

**Checked (code)**, `crates/kh1-hook/src/win/kh2slots.rs` (`SPECS`, `CYCLES`, `CASTS`,
`CAST_FINISHERS`, `kh1rows::OURS`) and `pose.rs` (`play_for`, `follow`):

| Already a KH1 slot of ours (baked with `kh1_bake`) | Requests |
|---|---|
| ground combo 151 / 152 / 154 and KH2's row picks | 0xC8..0xCB |
| air combo 181 / 182 / 183 and row picks | 0xCC..0xCE |
| walk / run, three slots each (field, battle, third) | 0x1, 0x2 |
| KH2 Fire / Blizzard / Thunder / Cure casts, finishers | 0x36..0x39, 0x3D, 0x83..0x86, 0x8A |
| KH2's ability swings on rows of ours | 0x200 + n |

| Still drawn over KH1's pass every frame (`follow` / `follow_play`) | How it's picked now |
|---|---|
| idle (field, battle) | `play_for` slots 0 / 1 → KH2 motion 0 (field variant on slot 0) |
| jump, high jump (takeoff + apex) | slots 8 / 9 and 12 / 13 → KH2 motion 3 (201 if it existed), **switched to the fall loop when KH2's arc falls** (`shared::physics::kh2_arc_falling`) |
| fall | slot 10 → KH2 motion 4 |
| landing | slot 11 → KH2 motion 5 |
| glide (slow, fast, two banks) | slots 75 / 76 → 205 (one group, so slow↔fast doesn't restart), 77 → 206, 78 → 207 |
| item use | `takeover::item_use_now()` (a wall-clock timer of `ITEM_CUT` 51 frames) → KH2 motion 9 |
| Gravity / Stop / Aero (Magic style KH2, base Sora) | `takeover::kh1_spell_now()` → Thunder's 62 / 64 (Gravity), Cure's 65 / 67 (Stop, Aero) |

Each of these costs a retarget of KH2's pose every frame, our foot IK (`feet::place`), and the
switching between KH1's pose and ours that the slot work exists to remove.

## 2. KH1's requests and slots for each one

**Checked (files)**: read from the user's `local/re/kh1/xa_ex_0010.mset` (request table = section 1;
entry bits 0..11 slot, 12..14 kind, 15 travel). **Checked (decomp)** for the kinds:
`0x1402A01E0` (the resolver) turns kind 0 into "put the weapon away", kind 1 "draw it", kind 3 slot
+0/+1/+2 (field / battle / third), kind 4 slot +1 in battle, kind 2 as is; it also passes the
request's own high bits through.

| Request | Entry | Slot | Kind | Frames | Events in KH1's slot |
|---|---|---|---|---|---|
| 0x0 idle | 0x4000 | 0 field, 1 battle | 4 | 120 / 170 | foot effects (type 4, a = 4) on bones 48 / 22 at 2 / 10 (slot 1: 52 / 110) |
| 0x4 jump takeoff | 0x2008 | 8 | 2 | 26 | foot effects at 4 (both feet), sound 5 at 2, voice 21 at 14.5 |
| 0x5 jump apex | 0x2009 | 9 | 2 | 14 | none |
| 0x6 fall | 0x200A | 10 | 2 | 40 | none |
| 0x7 land | 0x200B | 11 | 2 | 28 | **"may be left" (type 0) at 12**, landing foot effect at 0, sound 6 at 0 |
| 0x9 high jump takeoff | 0x200C | 12 | 2 | 34 | foot effects at 4, sound 7 at 4, voices 14 / 21 at 8.8 / 27.5 |
| 0xA high jump apex | 0x200D | 13 | 2 | 26 | none |
| 0xB / 0xC high jump fall / land | 0x200A / 0x200B | 10 / 11 | 2 | | (the plain jump's) |
| 0x3E item on himself | 0x203A | 58 | 2 | 82 | "may be left" at 74, sound 11003 at 30.8 |
| 0x3F item on a partner | 0x203B | 59 | 2 | 72 | "may be left" at 62, sound 11004 at 30.8 |
| 0x71 / 0x72 glide slow / fast | 0x204B / 0x204C | 75 / 76 | 2 | 120 / 60 | none |
| 0x73 / 0x74 glide banks | 0x204D / 0x204E | 77 / 78 | 2 | 60 / 60 | none |
| 0x76..0x79 | same as 0x71..0x74 | 75..78 | 2 | | |
| 0x3A Gravity (ground) | 0x1037 | **55** (Blizzard's) | 1 | 86 | "may be left" at 68 |
| 0x3B Stop / 0x3C Aero (ground) | 0x1038 | **56** (Thunder's / Cure's) | 1 | 68 | none |
| 0x87 Gravity (air) | 0x1050 | **80** | 1 | 78 | "may be left" at 68 |
| 0x88 Stop / 0x89 Aero (air) | 0x1051 | **81** | 1 | 68 | none |

So Gravity, Stop and Aero have **requests of their own** that share Blizzard's and Thunder's
slots: they can be redirected one by one without touching Blizzard / Thunder / Cure (the redirect is
per request). The casts are kind 1, so KH1 draws the Keyblade for them; everything else here is kind
2 or 4 and leaves the weapon alone. **Checked (files + decomp).**

Every one of these motions uses the same IK layout as KH1's other moves (11-12 helpers, joint
modes 1/2/3 on two bones each: the legs' ground-lift chains). **Checked (files).**

## 3. What in KH1 asks for them, and what depends on their timing

### Idle
- Request 0 comes from KH1's player step `0x1402A9740` when he stands still on the ground (blend
  10), and as the "action over" request `0x40000000` that KH1 queues after almost every action
  (landing, item, casts). Both resolve through the request table, so a redirect of request 0
  covers them. **Checked (decomp).**
- `0x14029F7C0` (the blended request) skips a request whose slot is already playing, and when walk
  starts out of idle it starts the walk at half the blend (no phase sync from idle). Walk↔run phase
  sync uses both slots' frame counts (already handled by the walk/run bake). **Checked (decomp).**
- What reads idle: our `pose::REST` / `REST_OPEN` take KH1's battle and field idles **from the file**,
  not from the actor's slot table, so a redirect doesn't change them. **Checked (code).**
- **The feet.** KH1's idle stands on uneven ground with its leg IK's ground lift (mode 2; the
  user's t348 trace, `kh1-anim-slots.md` §3.1). Today's drawn-over idle gets our own foot IK
  (`feet::place`). A raw bake is plain FK legs: **no foot placement on slopes or stairs** until the
  "hybrid legs" step (§3.1 of `kh1-anim-slots.md`: legs keep KH1's joint words, H8 / H9 keyed to the
  baked ankles) is built. Idle is where this shows most.

### Jump, high jump, fall, landing
- **Takeoff:** `0x1402AF9F0` asks for 0x10004 (plain), 0x10009 (with High Jump, ability bit 0x200
  of `0x142D5EC10`) or 0x100AF (actor flag 0x2000: a third set, `+0x3D0` = 0xAB; which situation
  that is wasn't found). It stores the set's base in `+0x3D0` (0, 5 or 0xAB). Bit 16 of the request
  is passed through by the resolver and ends up as the motion's "hold at the end" flag
  (`+0x160` bit 3) in the apply step `0x14029FC40`. **Checked (decomp)** for the call;
  **Assumption** (likely) for what bit 16 does, read from the apply step's flag arithmetic.
- **Apex:** `0x1402AF630`, each rising frame: when KH1's jump clock `+0x3B0` + 6 reaches the rise
  time `+0x3B8`, nothing is queued and the takeoff is still playing, it asks for base + 5 (apex)
  and queues base + 6 (fall). So KH1 switches to its apex animation **6 frames before the top**, and
  to the fall when the apex slot ends (14 frames for the plain jump, 26 for the high jump).
  **Checked (decomp).** Our physics writes `+0x3B0` / `+0x3B8` from KH2's arc every frame
  (`shared.rs`, **Checked (code)**), so these requests already fire on KH2's timing.
- **KH2's own rule** (`kh2_rig`, our rewrite): the jump clip plays while rising and the fall loop
  starts the frame he starts falling. That is why `follow_play` swaps KH1's apex for KH2's fall.
- **Fall:** request 6 (or base + 6) from the player step when KH1's state becomes 2 (walking off a
  ledge), from the apex queue, and from the ceiling bonk `0x1402AF3F0`. **Checked (decomp).**
- **Landing:** the player step, on the frame his state goes 2 → 0, asks for base + 7 with bit 24
  (action flag 4: he can't act) and queues `0x40000000` (idle, action over, blend 8). Flag 4 is
  cleared by the landing slot's "may be left" event (type 0, KH1's frame 12 of 28: `+0x1CC` bit 20,
  cleared in the animation step). So the landing slot's type-0 event **is** KH1's landing lock.
  **Checked (decomp + files).**
- KH2's landing (our controller, **Checked (code)**): buttons from the clip's reaction window
  (frame 8), a jump at any frame, the stick after `LAND_HOLD` = 14 frames (that 14 is from the
  user's video, not KH2's exe: **Assumption** in our own code).

### Glide
- Start: `0x1402B2D30` asks for 0x71 (blend 16) and sets state 0x15. Each frame KH1's glide step
  (decomp around `0x1402B2880`) re-picks: 0x73 / 0x74 when the stick turns him more than 30°
  either way, else 0x71 (speed ≤ 8) or 0x72 (faster), only while no blend is running.
  `0x14029F7C0` ignores a request whose slot is already playing. **Checked (decomp).**
- So if 0x71 and 0x72 both point at **one** slot of ours, KH1's slow↔fast switch stops restarting
  the clip, which is what `play_for`'s "group 5" does by hand today. Native, no hook.
- A second step (`0x1402B2010`, decomp line ~1430; looks like Neverland's flight, state 8) picks
  0x75..0x7B by its own rules. 0x75, 0x7A, 0x7B have no entry in Sora's own set (they come from the
  world's set). In Sora's own set 0x76..0x79 land on the glide slots. **Checked (files + decomp)**;
  which move actually uses 0x76..0x79 in Sora's own set (Superglide or flight) is **not confirmed**.
  Our set copies only redirect where the set's entry holds the motion ours stands for (by signature),
  so a world set's flight motions are not touched (**Checked (code)**, `kh1-anim-slots.md` §2).
- KH2's glide hover (208, a glide started with the stick neutral) has no KH1 request at all: it
  would need a request of ours (0x200 + n) asked for by the glide state, which belongs to the
  "KH2 Glide in KH1's glide state" item.
- The glide slots have no events (no sounds, no effects) in KH1. **Checked (files).**

### Item use
- KH1's command step `0x1402A3DB0`, command type 3: asks for 0x3E (on himself) or 0x3F (on someone
  else), blend 8, queues idle (action over), then calls `0x1402C2760`, which starts the item's
  effect as a task with a wait taken from a per-member byte table (`0x142D22D30` + 0x40 / + 0x41 +
  member × 2). **Checked (decomp).** That the byte is a wait in frames is an **Assumption**; but the
  effect is not tied to the motion's time or events, so a shorter KH2 item motion does not move the
  heal. (Whether the heal could then land after KH2's motion ends depends on that wait's value:
  to check in game.)
- What the motion drives: the "may be left" event (when he can act again: 74 / 62) and the item
  sound (11003 / 11004 at 30.8). **Checked (files).**
- Today we draw KH2's item motion by a wall clock and cut at our `ITEM_CUT` (51); a baked slot puts
  both on KH1's own clock. Today's draw follows the command (`item_use_now`), not the slot, so both
  0x3E and 0x3F get KH2's motion, timed by the clock (**Checked (code)**); a bake needs both.

### Gravity / Stop / Aero
- KH1's command step asks for 0x36 + spell on the ground and 0x83 + spell in the air (`0x1402A3DB0`
  and the air version), so Gravity / Stop / Aero are 0x3A / 0x3B / 0x3C and 0x87 / 0x88 / 0x89; the
  spell itself is started at once by `0x14028F140`, not by the motion. **Checked (decomp)** for the
  requests; that nothing of the spell reads the caster's motion events is an **Assumption** (KH1's
  spells are their own script objects).
- The casts already baked (`CASTS`) show how: `kh1_tracks(cast: true)` puts KH2's free windows as
  KH1's "may be left". Gravity's template is slot **55** (Blizzard's), Stop's and Aero's 56 / 81
  (Thunder's) and air Gravity's 80.
- Wisdom Form's version of this (the form's own cast motions) stays on the borrow path: bakes are
  base Sora only (`shared::on()`).

## 4. What a bake keeps and loses (applies to every item)

- **Kept (KH1's):** the request id at `+0x164` (event script opcode 322, `0x1402AF630`'s takeoff
  test, attack objects' `+0x58`), the state code, the clock, blends, the queue, foot effects and
  sounds from the template (rescaled to KH2's length), the weapon draw/put-away of the request's kind.
- **Lost in a raw bake** (`kh1-anim-slots.md` §3.1, the user's rule): our feet, `keyblade_teeth_up`,
  `kh1_wrist`, `flip_weapons`, the dream weapons' hold. The baked swings already live with this.
- **To check in our code** before each step: anything that takes a KH1 motion's length by its KH1
  slot must take ours instead (`kh2slots::frames_for`): `pose::slot_frames` (air cast "played
  through", s157), `takeover::ITEM_CUT`, `kh1_cut_frame`. **Checked (code)** that they exist; not
  yet whether each goes through `frames_for`.
- **Not scrubbed:** KH1 drives the motion time by hand only for requests 0x15 / 0x16 and 0xBF
  (`0x14029FF80`), none of ours. **Checked (decomp).**

## 5. Plan, one system per version

1. **Gravity / Stop / Aero** (smallest): six specs on 0x3A / 0x3B / 0x3C (templates 55 / 56 / 56)
   and 0x87 / 0x88 / 0x89 (80 / 81 / 81), KH2 motions 62 / 65 / 65 and 64 / 67 / 67, style Magic /
   Aerial Magic as `BASE_KH1_CAST`; drop the `follow` branch for base Sora.
2. **Item use**: 0x3E and 0x3F on KH2 motion 9 (templates 58 / 59), style Misc Animations; "may be
   left" from KH2's clip (its free window, as a cast), KH1's item sound rescaled or dropped where
   KH2's own plays; `ITEM_CUT` replaced by KH1's own action end.
3. **Fall and landing**: request 6 / 0xB → KH2 4 (loop), 7 / 0xC → KH2 5; the landing's "may be left"
   at KH2's reaction-window frame (question 2). Style Jump.
4. **Jump and high jump**: 4 / 9 → KH2's jump (held at its end), 5 / 0xA (apex) → **KH2's jump
   held for 6 frames** (the slot retimed to 6), so KH1's own apex request 6 frames before the top
   hands to the fall exactly at KH2's top. Removes the `kh2_arc_falling` swap. High jump uses
   KH2's plain jump (no 201 clip in KH2's files).
5. **Glide**: 0x71 / 0x72 (and 0x76 / 0x77) → one slot of KH2's 205; 0x73 / 0x74 → 206 / 207.
   Needs `kh2slots` to point two requests at one slot (one `Move` per slot today). Hover (208)
   waits for the glide-state item.
6. **Idle**: request 0 kind 4 → two slots in a row (field: KH2's field idle on 0's template;
   battle: KH2's idle on 1's), like `CYCLES` but two. Best after the **hybrid legs** (question 1).

Performance: about 20 more bakes at load on the loader thread (each a few tens of KB of heap);
per frame, the redirect loop grows by ~20 entries, and each of these moves stops costing a full
retarget plus foot IK while it plays. No new hooks.

## 6. Open

- Which move uses 0x76..0x79 in Sora's own set (Superglide or flight).
- The third jump set (`+0x3D0` = 0xAB, actor flag 0x2000).
- The apex request's blend length (the decomp loses that argument).
- The item effect's wait values, against KH2's item motion (50 frames, per our code's note).
- In game: how KH1's blend treats an FK slot next to its own IK motions (open since t349).

## 7. Built (after the tester's go-ahead, m1's answers)

The tester's answers: 1 "I don't want a hybrid layer just do (a) without baking on top of it" (open:
asked again, idle and the landing stay drawn over as before); 2 "same as kh1 for now" (KH1's landing
lock stays KH1's own); 3 "(b) kh2 voice plays if the toggle for kh2 voices is on".

- **Step 1, Gravity / Stop / Aero: already done** before this round (t563, `kh2slots::CASTS`
  entries 10..15: 0x3A / 0x3B / 0x3C on KH2's 62 / 65 / 65, air 0x87 / 0x88 / 0x89 on 64 / 67 / 67,
  by each spell's animation row). The `pose::follow` branch for them is left only for the borrow
  (a state KH1 casts nothing in). **Checked (code)**.
- **Step 5, glide slow / fast: already native** (the KH2-glide thread). At Glide KH2 our pre-step
  (`glide::pre_step`) replaces KH1's `0x1402B2880` and asks only 0x71 (205), 0x73 / 0x74 (206 / 207)
  and the hover request of ours, so KH1's slow / fast pair 0x71 / 0x72 never switches under KH2 and
  205 doesn't restart; at Glide KH1 KH1's own glide plays. **Checked (code)**.

### 7.1 Item use (step 2)

- `kh2slots::ITEMS`: KH2's motion 9 on 0x3E (template slot 58) and 0x3F (template 59), style Misc
  Animations. KH1's command step (`0x1402A3DB0`, type 3) asks them with bit 24 (action flag 4) and
  blend 8, queues the idle with `0x40000000` and blend 8, and starts the item's effect task
  (`0x1402C2760`), none of it tied to the motion. **Checked (decomp)**.
- KH1's "may be left" (type 0) is KH2's free state (range id 2 from frame 36 of 50, **Checked
  (files)**), so the stick and buttons take him from 36, as in KH2; the slot ends at 50 into KH1's
  queued idle (KH1's own action end).
- Sounds: KH1's item sound (track type 6 at 30.8 in slots 58 / 59) is left out of the slot; KH2's
  own sound 305 at frame 10 plays from the slot's KH2 events (`kh2slots::tick` →
  `kh2sound::event`), whatever `kh2_sounds` lists (`kh2sound::allowed`); KH2's voice 3 at frame 0
  only with KH2's battle quotes on (`kh2sound::kh2_voice`). No KH1 voice is started by the item's
  command step or its task (**Checked (decomp)**, `0x1402A3DB0` case 3, `0x1402C2760`).
- `takeover::item_use_now` ends with KH1's item slot (58 / 59) when the slot is built, no longer by
  the 51-frame wall clock (`ITEM_CUT` stays for the hand-off path, `kh1_cut_frame`).
- Per frame: nothing new (the slot is in KH1's table; the item's draw-over and its retarget are gone
  while it plays). Two more bakes at load (50 frames each).
- **Not checked**: in game; whether the item's heal (its task's wait, `0x142D22D30` bytes) lands
  before KH2's motion lets him go at 36.

### 7.2 The fall, and KH2's swap into it (step 3)

- `kh2slots::FALL`: KH2's fall (motion 4) on KH1's fall request 6, slot 10 as its template, style
  Jump; the High Jump's fall 0xB (also slot 10) points at the same slot (`kh2slots::ALIASES`).
  **Checked (files)**: both entries `0x200A`; KH2's 4 is 32 frames at 60 Hz with its loop point
  (`frame_return`) 6 at 30 fps, i.e. 12.
- **KH1's loop range.** KH1 loops a request without the hold bit (`+0x160` bit 3, from request bit
  16, `0x14029FC40`) over its whole motion: KH1's start (`0x1402A0350`) sets `+0x288` = 0 and
  `+0x28C` = its length unless `+0x160` bit 6 (a range of its own, KH1's setter `0x14029FA10
  (actor, start, end, third)`), and its step (`0x14029F0A0`) wraps the time from `+0x28C` to
  `+0x288` + the overshoot. The apply (`0x14029FC40`) clears bit 6 with each new request.
  **Checked (decomp)**. `kh2slots::loop_range` sets KH2's loop point there (12..32) once KH1 has
  started our fall (each frame KH1 poses him: one compare), so KH1 loops it as KH2 does.
- **The range leaked into the next motion (fixed, glide thread after m7).** `loop_range` runs from
  KH1's pose pass, and KH1's apply (`0x1402A0350`) calls that pass (`0x140391C70`, inside
  `0x140393B30` / `0x140394080` for a blended start) on the motion coming in before it writes the
  new slot to `+0x168`, after `0x14029FC40` cleared bit 6. `current` then still named the fall, so the
  fall's 12..32 went back on with bit 6, and a blended start keeps the range it finds (only the
  unblended start sets the whole motion). The next motion took the fall's loop: the tester's trace
  (f113654..f113834, Jump KH2, a neutral glide out of the High Jump's fall) has the hover 208 (100
  frames) playing 0, 2 .. 32, then 13 .. 32 again and again. **Checked (decomp, trace)**. Now
  `loop_range` does nothing while `+0x160` bit 0 is set (a request queued or being applied: set by
  KH1's request `0x14029F7C0`, cleared by its update `0x14029F0A0` after the queue step returns);
  the fall's range is set on KH1's next plain pose of it, one frame later, long before frame 32.
- **KH1's apex.** KH1's apex step `0x1402AF630` (from the rise, each frame) asks its apex (base + 5,
  blend 6.0, the float at `0x14042F93C`) when `+0x3B0` + 6 ≥ `+0x3B8`, nothing queued, its takeoff
  (base + 4) playing, and queues its fall (base + 6, blend 0). **Checked (decomp, exe)**. KH2's jump
  update (KH2 Steam `0x1403CB680`) instead swaps straight to the fall (`+0x118`, set to 4 by the
  jump's start `0x140422630` for the plain jump and the High Jump, whose motion 201 comes from
  `0x1403B59B0`) through `0x1403CB610`, blend 8.0, when `+0xF0` + 5 ≥ `+0xF4`; and on an early
  release (after 6 frames, more than 5 before the apex) at once, before the hop. **Checked
  (decomp)**. So at Jump KH2 (base Sora, our fall built) a wrapper on `0x1402AF630` (prologue
  `40 53 48 83 EC 20`, Steam only) runs KH2's rule: KH1's fall request with blend 8 at apex − 5,
  no apex motion; `shared::physics`'s early release asks the same at once
  (`kh2slots::fall_now`). Whatever else reads the request: only `0x1402AF630` itself tests
  base + 5 (**Checked (decomp)**, every `+0x3D0` + n in KH1).
- The landing stays drawn over (feet, open question 1). So that it blends from the baked fall
  instead of snapping, `pose::follow_baked` now keeps the baked frame's KH2 pose (body and Keyblade)
  as the last pose drawn; this also gives a blend into any drawn clip after a baked one (a baked walk
  or run into the drawn idle, which snapped since t389).
- Per frame: the loop check, the apex wrapper's few compares while rising, and one Keyblade
  evaluation on baked frames (for the blend). One more bake at load (32 frames).
- **Assumption**: KH2's arc clock and blend are in 60 Hz frames here as KH1's (the earlier glide /
  dodge work's reading). **Not checked**: in game; the third jump set (`+0x3D0` = 0xAB) is left to
  KH1.

### 7.3 Jump and High Jump (step 4)

- `kh2slots::JUMPS`: KH2's jump (motion 3, 26 frames) on KH1's takeoff 0x4 (template slot 8, 26
  frames) and KH2's High Jump (201, 64 frames) on 0x9 (template slot 12, 34 frames), style Jump.
  KH1's takeoff `0x1402AF9F0` asks them with bit 16 (hold at the end, **Assumption** (likely), read
  from the apply's flag arithmetic, §3). **Checked (files)**: KH2's 3 and 201 leave the ground at
  frame 4 (their frame trigger 0), 201 has no range id 49 (`+0x194` bit 4, which would keep KH2's
  swap off), only "air" from 8.
- KH2's jump start (`0x140422630`, `0x1403B59B0`, **Checked (decomp)**) plays 3, or 201 with High
  Jump, and sets the fall motion `+0x118` = 4 for both, so the High Jump's 201 is cut by the same
  swap 5 frames before the apex (§7.2), not played to its end as the OG rewrite's rig (`kh2_rig`,
  "plays to its end however the rise goes") has it: the exe's rule is taken.
- KH1's apex requests (5 / 0xA, slots 9 / 13) are never asked at Jump KH2 (the apex wrapper), so
  they are not built; the plan's "jump held 6 frames" apex slot isn't needed.
- KH1's takeoff sounds and foot effects stay at KH1's own frames (not stretched to 201's 64): the
  takeoff is frame 4 in both. KH2's own jump sound (303) isn't played (not in `kh2_sounds`' list),
  as before.
- `pose`'s drawn-over swap to the fall (`shared::physics::kh2_arc_falling`) stays only for when the
  slots aren't built.
- Per frame: the takeoff's retarget and our foot IK while he rises are gone. Two more bakes at load
  (26 and 64 frames).
- **Not checked**: in game; KH1's blend between its takeoff and our slot (KH1's takeoff request's
  blend, from `0x1402AF9F0`).

## 8. KH1's own leg IK on the baked slots (after m2's "(a) go ahead")

The tester's answer to m2's question: "(a)": KH1's own leg IK on the baked slots (feet on slopes and
stairs), idle and the landing **not** baked yet (they stay drawn over, with our foot IK). This is not
a layer of ours: KH1's own bone pass runs its own leg rig on our motion, the way it runs it on every
one of its own motions; only the motion data is ours. `kh1_data::legs`, `bake::baked(.., kh1_legs)`,
ini `kh1_leg_ik = on` (default; `off` = the plain FK legs of t349..t627).

### 8.1 KH1's leg rig (Checked (files + decomp + exe))

- **Every one of Sora's 118 motions has it, twice** (`examples/bake_legs.rs` over `xa_ex_0010.mset`:
  no slot with other than two legs): bones 16..20 on helper 9 (A / P field 10) and 42..46 on helper 8.
  Hip 16 / 42 `lo` 1 (aim at the helper), thigh 17 / 43 mode 2 (two-bone IK, no pole), shin 18 / 44
  (solved with the thigh), ankle 19 / 45 (`P` = the helper, keep position, world parent), foot 20 / 46
  `lo` 2 (the helper's turn). Hip and thigh sit at one point (thigh bind translation 0), the shin and
  ankle on their parent's x (29.63, 32.01), scale 1. The shorts' panels hang from the thighs (17: 26,
  30, 34, 38; 43: 52, 56, 60, 64), the hip carries only the thigh.
- **The pass** (Steam bone pass `0x1403900B0`, solver `0x14038FC20`, helper build in `0x140391C70`
  with `0x1401DB080`): the matrix helpers are D3D rows (glam's columns), `0x1401DB4D0 (out, a, b)` is
  `a * b` in glam's terms (it calls `0x1400E5A20` with a, b, whose rows are b's rows times a,
  **Checked (exe)**), `0x1401DAD10 (out, m, v)` is `m * v`, the x turn `0x1400E6AC0` makes `Rx * m`
  (rows of m times Rx, **Checked (exe)**). So, in glam's terms:
  - hip: x toward the helper's position from the bone's sampled translation (parent space), y the
    parent's y made square to it, z = x × y, then `× Rx(its sampled rotation x)`;
  - thigh: the solver's frame (`d` to the target, `p = (-d.y, d.x, 0)`, `z = d × p`) `× Rx(thigh's
    rotation x) × Rz(α)`; shin `T(l1) × Rz(θ) × Rx(shin's rotation x)`, `l1` / `l2` the shin's and
    ankle's bind x times the sampled scales, the knee angle from the law of cosines, sine positive
    (bend flag 0 = mode 2 − 2);
  - ankle: at `world(shin) × bind(ankle)`, turned by its own sampled Euler (X→Y→Z) against helper
    matrix 0, the model;
  - foot: the helper's rows (normalised), at `world(ankle) × its sampled translation`;
  - helper k: `matrix[record parent] × (scale, Euler X→Y→Z, translation)`, its channels sampled into
    the motion's own record every pass (`0x140391C70`), so a record's parent is the motion's.
- **Ground lift** (the thigh's step): with the instance's `+0x14 & 3` and the thigh listed in the
  model's lift list, KH1 casts down
  (`0x140193230`) from 32 above the helper's position (the ankle's bind x) and, where the floor is
  above "helper − that height", lifts the target onto it (`+0x14 |= 0x200000`). On Sora this is on:
  the t348 trace, KH1's own idle on uneven ground, `+0x14 = 0x00200001`, ankles 17.4 / 29.8 above
  the floor under his centre. The list is in the model header's info block (header `+8`, an offset
  from the header; the pass reads it through the instance's model, `*(*instance) + 8`): four heights
  at `+0..+0xC`, the head bone at `+0x10`, the lift bones (bone + 1) at `+0x90..+0x9C`. Sora's
  (`xa_ex_0010.mdls`, **Checked (files)**): heights 17.434 ×4 (his ankle's bind height, 17.43),
  head 151, lift bones 44, 18, 44, 18: the thighs 43 and 17. **Assumption**: that the instance's
  bits 0..1 are on while our slot plays as while KH1's own does (they are the instance's, not the
  motion's; who sets them wasn't found in the decomp).

### 8.2 What the bake writes for a leg

`legs::find` keeps a leg only when the template has exactly that layout and its helper is used by
no other bone and no helper hangs from it. Then, per frame of the retargeted pose (`legs::frame`):

| What | Keyed to |
|---|---|
| helper 8 / 9 | parent changed to matrix 0 (the model, as the poses), scale 1; rotation = the pose's foot (20 / 46) turn in the model, translation = the pose's ankle (19 / 45); the template's channels on it dropped |
| hip 16 / 42 rotation x | the roll that makes KH1's thigh turn exactly as the pose's thigh (`Rx = hip0⁻¹ × thigh × Rz(-α)`), the thigh's own rotation x left at its bind, as KH1's own motions do (slot 0's hip word `0x02000029` static, slot 5's `0x00000029` keyed, the thigh's static in both) |
| shin 18 / 44 rotation x | the pose's shin roll about its own x |
| ankle 19 / 45 rotation | the pose's ankle turn in the model |
| foot 20 / 46 translation | as in the plain bake (its local, the bind) |
| the other channels of these bones | the bind (nothing written; KH1's IK and the helper place them) |
| their joint words | the template's (the hip's static bit only when nothing on it moves) |

Arms, spine and root stay plain FK as before (the tester asked for the legs).

**Checked (files)**, `examples/bake_legs.rs` on the user's files (KH2 walk 1 / run 2 / swings 151, 152,
154 / air 181 / fall 4 / jump 3 / High Jump 201 / item 9 / Blizzard 56 on their KH1 templates): with
KH1's pass replayed (`legs::build`, flat ground) every joint within 0.19..0.69 units of the pose (the
plain FK bake: 0.19..0.30; the worst, 0.69, a nearly straight knee in the item motion), thigh and shin
within 0.06..1.35°. Unit tests `legs::tests` (bent legs of any roll rebuilt within 0.01).
- **What differs from the plain bake on flat ground:** the hip bone 16 / 42 is KH1's aim at the
  ankle, 53..84° off the retarget's hip, which turned it with the thigh. It carries only the thigh,
  which is placed as before; any skin weights on the hip itself now move as in KH1's own motions.
- **Feet lower than KH1's ankle height**: the baked ankles stand at 17.4 on flat ground like KH1's bind
  (17.43), but some frames go lower (swings 151 / 154: 14.8, soles 2.6 under the floor). There KH1's
  lift raises the ankle to its own height (17.434, the lift list's), as it does in its own motions.

### 8.3 Our foot IK (`win/feet.rs`) on the baked slots

It never ran there and still doesn't: `pose::follow` returns at `follow_baked` (no pose drawn) before
`feet::place` (it lives in the drawn path, `follow_play`). So on a baked slot only KH1's lift acts;
on a drawn clip (idle, landing, glide, the drawn casts) only ours does, as before. **Checked (code).**

### 8.4 Cost

- Per frame: nothing of ours. KH1's pass does on our slots what it does on its own motions: two aims,
  two two-bone solves and, with the lift bits, two floor casts, instead of ten FK bones.
- At load: a few hundred more helper keys per baked motion (169..731 in the list above, 16 bytes
  each) and one leg frame worked out per pose frame.

### 8.5 Open

- In game: the lift on stairs and slopes in walk, run and the swings; KH1's blend into and out of our
  slots now meets KH1's own IK list on the legs (the blend pass re-solves the list, `0x140392560`).
- Idle and the landing: still drawn over (the tester's "(a)").
- Who sets the instance's lift bits (`+0x14 & 3`).

### 7.5 The helper channel count (t633)

- A KH1 motion's helper channel count is the header's `+0x20` (the pose pass `0x140391C70` samples that
  many 6-byte helper channel records from `+0x24`). **Checked (decomp)**. The bake rebuilt the helper
  channel list (travel replaced, the foot helpers' template channels dropped and ours added) but left
  `+0x20` at the template's count, so KH1 read too few (the last leg keys never sampled) or too many (past
  the list) channels. It is now written with the list's length. `bake::tests::bakes_a_chain` checks it.

## 9. Foot IK off (the tester's m3: "lets actually have the foot iks off we'll work on this later when we do soras cloth animations")

- One switch, `foot_ik` in kh1-kh2.ini, **off by default** (a new key, so an ini that already has
  `kh1_leg_ik = on` turns off too). Off:
  - the baked slots get plain FK legs (`kh1_leg_ik` is read as off; the `kh1_data::legs` code stays
    for the cloth work), so KH1's ground lift has no mode-2 bone to act on in our motions;
  - our foot placement on the clips drawn over KH1's pass (`feet::place`) runs with `active = false`,
    so any lift or drop it held eases out in a few frames and then it returns without touching the
    pose.
- `foot_ik = on` brings both back (KH1's leg IK on the bakes only with `kh1_leg_ik = on` too; read at
  load, the bake happens then).
- Nothing else of ours places feet: `feet::place` is the only caller of KH1's floor ray
  (`0x1401910E0`) in the hook (**Checked (code)**). KH1's own motions (its own slots, when KH1 shows
  him) keep KH1's own leg IK and ground lift: that is KH1's, not a layer of ours.
- Cost: none per frame (one bool read; `place` returns at once once eased out).

## 10. Idle baked (the tester's m3: "just make sure every animation is baked and on the native system")

- KH1's idle is request 0, entry `0x4000`: slot 0, kind 4, which KH1's resolver (`0x1402A01E0`)
  plays as the slot in the field and slot + 1 in battle (§2; **Checked (files + decomp)**). It is
  built like walk and run (`kh2slots::CYCLES`, now with a slot count per request: two for kind 4,
  three for kind 3): KH2's field idle (motion 0's field variant, 180 frames at 60 Hz) on KH1's
  slot 0 as template, KH2's battle idle (motion 0, 300 frames) on slot 1, the request pointed at the
  first, KH1 adding 1 in battle itself. Style: Misc Animations (as KH1's slots 0..7 always were,
  `shared::slot_kind`). **Checked (files)**: `P_EX100.mset` motion 0 300 frames (no loop point, so
  KH1's whole-motion loop is KH2's), field variant 180.
- Everything that asks for the idle goes through request 0: KH1's player step when he stands still,
  the "action over" request `0x40000000` it queues after landings, items, casts and reactions, and
  scripts that set it. All of those now play KH2's idle (as walk and run already did), and the
  weapon stays as the request's kind leaves it (kind 4 draws or puts away nothing).
- **Longer than a byte of keys.** KH2's battle idle is 300 frames; a KH1 channel's key count is a
  byte. The bake no longer refuses motions over 254 frames (up to 4096): a channel that needs more
  than 255 keys has its tolerance doubled until it fits (`bake::reduce`; the key time,
  `frame × 256 << 8`, fits an i32 far past that). On the user's files the battle idle bakes
  within 0.36 units of the pose, no channel needing a looser tolerance (`examples/bake_legs.rs
  0:1`).
- `pose::play_for`'s idle (slots 0 / 1) stays only as the fallback when the slots aren't built.
- Feet: plain legs (§9).
- Cost: none per frame (one more redirect, two fewer retargets while he stands); two more bakes at
  load (180 and 300 frames).
- Not checked: in game; scripts and cutscenes that ask KH1's idle now show KH2's (Misc Animations
  KH2), as they already did for walk and run.

## 11. The landing baked (m3)

- `kh2slots::LAND`: KH2's landing (motion 5, 26 frames at 60 Hz, reaction window from 8, free from
  12; **Checked (files)**) on KH1's landing request 7, slot 11 as template (28 frames); the High
  Jump's landing 0xC (also slot 11, `0x200B`) points at the same slot (`ALIASES`). Style: Jump (as
  KH1's slots 8..13, `shared::slot_kind`).
- KH1's player step asks the landing (base + 7, bit 24: action flag 4) when his state goes from
  the air to the ground and queues the idle (`0x40000000`) (§3, **Checked (decomp)**). The lock is
  KH1's own, as the tester asked in m1 ("same as kh1 for now"): the template's events stay at
  KH1's own frames (style Jump, not stretched: `make`'s `r = 1`), so its "may be left" (type 0) at
  frame 12 clears action flag 4 there, and its foot effect and sound 6 play at 0. KH2's own free
  frame is 12 too, its reaction window 8 (not used: KH1's lock).
- KH2's own landing sounds play only where `kh2_sounds` lists the motion (`kh2sound::allowed`), as
  for the jump.
- No loop point is set (KH2's `frame_return` lies past the motion's end), so KH1 plays it once and
  goes on to its queued idle (now ours too, §10).
- `pose::play_for`'s landing (slot 11) stays only as the fallback.
- Cost: none per frame; one more bake at load (26 frames).
- Not checked: in game.

## 12. Audit: what of base Sora is still drawn over KH1's pass (after §10, §11)

Read from `pose::follow` / `follow_play` / `play_for`, `kh2slots` and `kh1rows` (**Checked
(code)**). `follow` asks `kh2slots::kh2_motion(slot)` first, so every built slot of ours is
KH1's own pass (`follow_baked`, nothing drawn) whatever else `follow` would have drawn for it.

| Move | Now |
|---|---|
| Idle, walk, run | baked (§10, `CYCLES`) |
| Jump, High Jump, fall, landing | baked (§7.2, §7.3, §11); KH1's apex slots 9 / 13 are never asked at Jump KH2 (its apex step is KH2's swap, §7.2) |
| Ground and air combo, KH2's ability rows, Aerial Recovery, Aerial Dodge, Upper Slash / Finishing Leap / Horizontal Slash | baked (`SPECS`, `kh1rows::OURS`) |
| Glide slow / fast / banks, hover | baked (`GLIDES`, `kh1rows` "Glide hover"): no version needed |
| Guard press, guard repel, Counterguard, dodge roll, Quick Run and its stop | baked (`kh1rows`, `ROLL`) |
| Casts (Fire, Blizzard, Thunder, Cure, Gravity, Stop, Aero; ground, air, finishers), Reflect | baked (`CASTS`, `CAST_FINISHERS`, `kh1rows`) |
| Item use (self, partner) | baked (`ITEMS`, §7.1) |
| Flinches (ground front / back and their alternates, air), clash recoils (ground, air) | baked (`REACTIONS`, `RECOILS`) |

Still drawn, only as fallbacks: `play_for` (a slot whose bake failed or whose request isn't in a
world's set), the item and Gravity / Stop / Aero branches of `follow` (a state KH1 casts nothing
in), and KH2's controller pose (`takeover::POSE`) for the borrows left: a native start that
didn't take (`shared` falls back to `SQUARE` / `AIR_JUMP` / `QUICK_RUN`), a KH2 command chosen in
a state KH1 can't cast from (`combat::command` → `shared::want_kh2`), and Drive Forms (not base
Sora: bakes are base Sora's only).

Not baked, KH1's own animation at every style:
- **KH1's knock-backs** (ground 0x4C / 0x4D, slots 65 / 66, 72 frames in place; air 0x50 / 0x51):
  KH2's are two motions (the blow 16 / 18, then its landing 17 / 19 when he touches down), and
  KH2's launch arc isn't KH1's yet (the hit-reactions thread's plan step 3). A question for the
  tester.
- **KH1's big clash recoil** (0x6F, slot 73): the tester's own pick, KH1's (recoil thread, answer 1).
- **Ledges, vines, ladders, poles, bars, swimming, flight, the carpet and slider, Mermaid Kick,
  summons, KH1's specials, field actions, cutscenes**: KH1-only moves. KH2 has a counterpart only
  for the ledge (hang 7, 190 frames, and climb 8, 30 frames; `P_EX100.mset`, **Checked (files)**);
  KH1's ledge slots aren't identified yet. A question for the tester.

## 13. KH2's frame triggers as KH1 motion events (what-is-still-missing m11, item 6)

Before: KH2's sounds (frame trigger 8), voices (13 / 14) and effect elements (1) on our slots were
kept Rust side, and `kh2slots::tick` read them each frame between KH1's last and current time.
Now they are keys of the baked motion's own event list, and KH1's own motion pass fires them.

### 13.1 How KH1 fires a motion's edge events (Steam, Checked (decomp))

- `0x14029FB30 (actor, from, to)` is KH1's edge pass. The animation step `0x14029F0A0` calls it
  with the frame's step (`[t, min(t + dt, length))`), after a loop with the rest
  (`[loop start, loop start + rest)`), and when a blend ends with `[start, start + overshoot)`:
  while a blend runs (`+0x170` > 0) the time is held and no event fires. `0x14029FF80` (the
  states whose frame follows a value, e.g. the glide's 0x15 / 0x16) calls it too.
- It asks `0x140394900 (instance, motion, from, to, out, 8)` for the keys of type 4 (foot) and
  type 6 (sound) of ids 0..3 with `from <= time < to`. For each type and id only the **first**
  group of that type and id in the list is read. The out buffer holds 8 keys: the caller walks
  every key counted, so more than 8 in one call reads past it.
- Type 4 keys go to `0x1402C2660 (actor, a, b)` (unless `0x142867364` bit 0x100); type 6 keys go to
  `0x140295420 (actor, a, b, c)` (unless `0x142867364` bit 0x400). `0x1401A22B0` sets bit 0x400 with
  a stop of every sound; `0x1401A2130` / `0x1401B02C0` clear it (Assumption: an event or cutscene).
- `0x140295420`: after its own checks (`0x142867364` bit 0x200, the actor's mute bits `+0x9C` by
  `c`, `0x14017AF20`, the actor's side), `a` 0 plays sound `b`, `a` 0x19 his voice `b`, `a`
  1..0x1D a material sound through the table at `0x140528A87` (`0x140295B40`). The table's
  entries end at `a` 0x1D (Checked (exe)).
- Prologue of `0x140295420`: `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20` (Checked (exe)).

### 13.2 KH2's trigger timing (Steam KH2, Checked (decomp))

`0x14040E960 (triggers, object, from, to)` fires a trigger at frame F when `from < F <= to`, or
when `to` < 0 and F = `from`. The motion update `0x1403C6740` calls it with the frame's step, and at
a blend's end with `(start, -1)` then `(start, time)`; a motion started with no blend
(`0x1403C8A40`) fires its start frame at once. So KH2 holds its triggers through a blend too.

### 13.3 What the bake writes

- `kh2slots::kh2_trigger_keys`: each sound, voice and effect trigger of the clip as a type-6 key of
  ours: `a` = `KH2_SOUND` 0x81 / `KH2_VOICE` 0x82 / `KH2_EFFECT` 0x80 (past KH1's codes), `b` the
  KH2 motion, `c` the sound id / voice / element. A hit reaction's KH2 voice is left out as before
  (KH1's hit pass says his hurt line).
- Its time: KH2's frame F less 1/64 (`kh1_data::slot::kh2_trigger_time`), so KH1's `from <= t < to`
  fires it on the same update as KH2's `from < F <= to` with whole-frame steps (60 and 30 fps).
  Frame 0 stays 0: KH1 fires it on the motion's first update, or at its blend's end, as KH2 does.
  The slot's time is KH2's frame (the bake keeps the clip's length at 60 Hz).
- `kh1_data::slot::add_sound_keys` puts them in the template's first sound group (a new 6.0 when
  there is none), since KH1 reads only that one; a key that would make more than 8 foot and sound
  keys within 2 frames is left out (logged).
- The hook `kh2slots::sound_event` wraps `0x140295420`: our codes play through `kh2sound::event`
  and `kh2fx::event` on Sora, as `tick` did (their own gates: `kh2_sounds`, Battle Quotes,
  `kh2_effects`); KH1's own keys go to KH1's handler unchanged. Steam only; elsewhere the old
  per-frame read stays (`Move::events`).
- The slot cache's bake format is 2, so caches from before rebake.

### 13.4 What changes in play

- Same frame as before on an ordinary update; frame-0 triggers now wait for KH1's blend into the
  slot to end (before: at the blend's start), as KH2's do.
- During KH1's sound-off state (`0x142867364` bit 0x400) they don't fire, as KH1's own don't.
- `kh2slots::tick` now only notes which of ours plays (`PLAYING`, for `kh2fx::tick`, the trace and
  the Frame Advance panel) and puts back forced picks whose move is over.
- KH2's footsteps (trigger 2) aren't written: a baked slot keeps the template's own KH1 foot
  effects and floor sounds, rescaled (`kh1_tracks`), as before.

### 13.5 Effects: one key per frame, from the moveset's own package (Valor, t-vfx)

The tester (what-is-still-missing, after m13): "no effects" in Valor on KH1's own states.

- **Why.** The wrapped handler sent every effect key to `kh2fx::event`, which starts the element
  from **Sora's** package and only for the motions `kh2_effects` lists. Valor's motions (151..155,
  181..184, 40, the arrival 53) were not listed, so nothing played; and Valor's element numbers are
  its own archive's (`P_EX100_BTLF.a.us`), so even a listed one would have started Sora's element
  of that number. The old controller path sent a form's effects to `form::effect`
  (`kh2fx::form_event`, Valor's package); the Steam key path didn't know the slot was Valor's.
- **And the 8-edge cap.** Sora's Drive motion 52 starts 13 elements and a voice at frame 0, Valor's
  arrival 53 fifteen elements (1..12, 100..102) at frame 0 (Checked (files), `P_EX100.mset`,
  `P_EX100_BTLF.mset`). KH1 reads 8 edges a call (§13.1), so `add_sound_keys` left 6 / 7 of them
  out: the Drive's limb glow and Valor's arrival light were cut.
- **KH2 (Steam KH2, Checked (decomp)).** `0x14040E960` fires every trigger of the frame in the one
  update (no limit). Trigger 1 (`0x14040DD70` case 1) starts the element from the effect package of
  the object whose motion it is (`+0x148`, `0x1402C6B30 (package, id, 1, 1)`). Valor's weapon
  motions (`W_EX010_BTLF_R` / `_L`) carry no trigger 1, only sounds (trigger 8) (Checked (files));
  all of Valor's elements are on its body motions: 40 [0], 53 [1..12, 100..102], 154 [16 at 64],
  155 [13, 14 at 0; 15 at 42], 183 [17 at 0; 18 at 26; 19, 20 at 28], 184 [21..26].
- **Now.** Effects are one type-6 key per frame of the clip: `a` = `KH2_EFFECTS` 0x83, `b` the KH2
  motion, `c` = the frame | 0x10000 when the slot is the form's (`Move::form`, Valor's moveset)
  (`kh1_data::slot::kh2_effects_value`). The handler (`kh2slots::effects_at`) takes that frame's
  effect triggers from the moveset's own clip and starts each: Valor's through `form::effect`
  (Valor's package, its anchors: the right Keyblade's blade binding, the left one's on
  `leftblade`'s owner, `kh2dpx::form_anchor`), Sora's through `kh2fx::event` as before. Sounds and
  voices stay one key each (`KH2_SOUND` / `KH2_VOICE`). The slot cache keys on the tracks, so the
  slots whose keys changed rebake once.
- KH2's frame trigger 31 (52's frame 0 [1] and 20 [9], Valor's 41 [5], ...) is the pad's
  vibration: §13.6. Not checked in game.

### 13.6 KH2's pad vibration on KH1's vibration player (what-is-still-missing, answer 9 (a))

The tester (after m14): "9. (a) if rumble is on it should work then".

- **What trigger 31 is (Checked (decomp + exe)).** KH2's trigger pass (Steam KH2 `0x14040DD70`
  case 0x1F) calls `0x1403EEFC0 (param)`: a `bsearch` of the byte id in the `vibr` table of
  `00battle.bin`'s `vbrt` (a BAR of type 1, loaded by `0x1403F4C30` → `0x1403EEF80`), the entry's
  index byte naming the index-th pattern entry (type 0x2F, `0x1403A6420`), handed to
  `0x14019AE60`, which starts it in a task (`0x14019B1D0`) that writes the pad's two motor bytes
  (`0x140BF3270 + 0x48 / 0x49`, `0x14039B760 (0)`). No other combat code of KH2's calls it (its
  other callers `0x14020FDC0`, `0x1402DA350` are outside combat).
- **The patterns (Checked (files), the user's `00battle.bin`; `kh2_formats::battle::Vibration`).**
  "VIBD", two channels of (time in 30 Hz ticks, level) keys: 1 ch0 (0,255)(12,255); 2 ch0
  (0,255)(1,255); 3 ch1 (0,128)(4,160); 4 ch1 (0,128)(12,128); 5 ch0 (0,255)(2,255); 6 ch0
  (0,255)(30,255), ch1 (0,255)(30,160)(60,0); 7 ch0 (0,255)(15,255), ch1 (0,255)(10,255)(15,0);
  8 ch0 (0,255)(4,255); 9 ch0 (0,255)(12,255).
- **KH2's curve (Checked (exe), the step's instructions at `0x14019B290..0x14019B30F`).** The
  clock starts at 0 and adds the raw frame delta (`0x140717484`, 1.0 a 60 Hz frame) × 0.5 each
  step; a channel whose first key is at or before the clock writes `level(k + 1) × (t − t(k)) /
  (t(k + 1) − t(k))` for the first key k + 1 at or after it, truncated (so each segment rises from
  0 to its end key's level), nothing past its last key; the step the clock reaches the last key
  time plays `max − 1` and ends.
- **Which moves carry it (Checked (files)).** Every one of Sora's and each form's reactions 10..15
  (pattern 3), 16 / 18 (4); base Sora's recoils 41 / 43 (5), guard repel 42 (1), Drive 52 (1 at 0, 9
  at 20), the casts 56..61 / 71..73, Explosion 166 (9), Finishing Leap 167 (8), Counterguard 171 (2),
  Magnet Burst 197 (8); Valor's 41 / 43 (5), 42 (1); Wisdom's casts and 151 (8), 154 (8, 2, 8), 181
  (8). (Master, Final, Limit and Anti have their own; not baked yet.) No weapon motion carries one.
- **KH1's vibration (Checked (decomp)).** `0x14028F8D0 (n)` starts pattern n of `btlvibe.dat`
  (`0x142D36D60`, loaded by `0x14028B7C0`) on pad 0 through `0x1401E1400 (pattern, pad)`; the task
  20001 (`0x1401E1900` → `0x1401E1500`) steps pads 0 and 1 by the frame scale `0x14233FBDC`: a
  pattern is `0x14`, `+4` bit 0 loop, `+8` / `+0xC` the channels' entry offsets, `+0x10` / `+0x14`
  their counts, entries `u8` frames / `u8` level, each going from the entry before's level to its
  own over its frames, the first written as it is; at the end the bytes `0x14298C9B0 + pad × 0x70
  + 2 / 3` and the pattern are cleared. The pad read (`0x14028B0B0`) sends them (`0x1400DB010`:
  channel 1 as the level, channel 0 on above 0x7F) only while `0x1402FB030` (the config `+0x10`,
  Config's Vibration row: 0 on) is 0, and clears them every frame.
- **Now.** Each trigger 31 of a baked slot's clip is a sound event key of ours (`ks::KH2_RUMBLE`
  0x84, `c` the pattern id, at the trigger's frame as the others, §13.3). The wrapped handler
  (`kh2slots::sound_event`) sends it to `rumble::play`, which writes KH2's pattern once in KH1's
  layout for the frame scale it fires at (`kh1_data::vibe::kh1_pattern`: one entry per game frame,
  each the level KH2 writes on that step) and starts it on pad 0 with KH1's `0x1401E1400`
  (its first bytes checked). KH1's own task steps it, and KH1's pad read sends it only with
  Vibration on. A test steps both players (`vibe::tests`): KH1's levels equal KH2's from the
  second step on, at 60 and 30 fps; KH1's first step shows the second step's level (KH1's player
  writes its first entry twice). Steam only. One rebake of the slots with a trigger 31.
- **Assumptions.** KH2's channel 0 / 1 (`+0x48` / `+0x49`) are KH1's 0 / 1 (`+2` / `+3`): KH2's own
  send to the pad wasn't traced, KH1's turns channel 0 on or off and gives channel 1 its level.
  KH2's motor bytes read 0 after its pattern ends (its task stops writing them).
- **KH1's hit vibration stays (what-is-still-missing m15, answer 9: "for vibration just allow kh1
  in those situations, unless that gets rid of vibration on kh2s moves").** KH1's hit pass
  (`0x14029BBD0`, called from the player's think `0x1402A43A0`) starts its own `btlvibe.dat` pattern
  through `0x14028F8D0` only for Sora (member id 0) and only when it ran a reaction handler this
  pass: 1 for a flinch (3), 0 for a knock-back / blow (4 / 5), 6 / 5 for the clashes 6 / 7
  (**Checked (decomp)**). `0x1401E1400` keeps one pattern per pad and a start always replaces the
  one playing (it clears the pad's slot and sets the new pattern, no priority test; **Checked
  (decomp)**), so whichever starts last plays. KH1's other starters are event / script commands
  (`0x1401AC890`, `0x14039F330`, `0x1402BCAD0`) and `0x1402352A0`; nothing of KH1's vibrates on
  Sora's own hits on enemies (**Checked (decomp)**, every caller of `0x1401E1400` and
  `0x14028F8D0`). So:
  - a hit with a reaction: KH1's pattern starts in the think, the reaction slot's KH2 pattern
    (trigger 31 at its frame 0) after it in the motion pass, and replaces it: KH2's plays whole;
  - a hit during one of KH2's own moves (Explosion, a cast, ...) wipes that move's pattern only
    when KH1 runs a reaction for it, that is when the hit cuts the move into a reaction or recoil,
    whose own KH2 pattern then starts; hits KH2 doesn't react to (Drive, Reflect, dodge
    invulnerability, guards) are no-effect records (`hurt.rs`), which run no handler and start no
    KH1 vibration;
  - a second hit while a reaction plays starts KH1's pattern again; KH2's comes back only if
    KH1's reaction restarts the motion.
  No code change. **Assumption:** the motion pass runs after the think in Steam KH1's actor frame
  (the order traced on Epic, `0x140295E50`: think, physics, matrix, animation; not re-traced on
  Steam). Not checked in game.


## 14. KH2's motion ranges as KH1 motion events (what-is-still-missing, item 5)

Status at t867, before any code. What still reads KH2's ranges at runtime, how KH1's motion events
can carry them, and the plan.

### 14.1 What still reads KH2's ranges (Checked (code), t867)

- **Each frame Sora is posed** (`kh2slots::frame`): the Finishing Plus window (range 50,
  `finisher_window`), KH1's own casts' window at Magic Combo KH2 (`formmagic::kh1_cast_window`),
  the Counterguard window (`combomods::counter_window`), the invulnerable ranges (range 27,
  `invulnerable_step`, t860: KH1's no-damage timer does the work, but the range is read here).
- **Each frame in KH1's physics step** (`shared::physics`): the no-gravity hold (range 3 / 24),
  rise start, the forms' lift-off, the edge stop and ground hold / Ledge Carry (range 29),
  allow-movement steering (range 41), root travel in the hold. Turn to target (range 28,
  `turn_to_lock`) before it. The Keyblade re-arm (`keyblade_rearm`, the attack windows) each frame
  KH1 has him.
- **On a command only:** the cast checks in `combat::command_step` (`cast_attack_window`,
  `cast_magic_window`: range 11 / 35; `cast_finisher_open`: range 50), Valor's Square windows
  (`combomods`), the reaction command's first frame (`takeover`). `formmagic::cast_cut` reads the
  free range only on a fresh press during a cast (no cast motion has one).
- **Already KH1's own data:** KH2's free range as KH1's "may be left" (type 0) and the cast's combo
  window as KH1's combo (type 5), baked once into our slots (`Leave`, `cast_follow`).
- **Correction to the t793 list:** the jump's apex swap (`kh2_apex`), the Aerial Dodge's apex and
  clock (`dodge_apex`, `dodge_clock`) and the glide clock (`glide::kh2_clock`) are KH2 timing
  rules, not motion ranges. They aren't part of this item. The guard windows come from KH2's
  sweep (`takeover::kh2_hits`) and go with the body-hit work.

### 14.2 How KH1 reads a motion's events (Checked (decomp), Steam KH1)

- **Levels.** KH1's plain pose pass (`0x140391C70`) clears the top half of the instance's level
  word (`+0x14`, bits 16..31) and asks its level reader (`0x140394770`, by exact type and id: the
  last key at or before the time, "on" in `a`) for 1.0..1.3 (bits 16..19), 0.0 (bit 20), 5.0
  (bit 22) and 7.0 (bit 23). Nothing else. Bits 21 and 24..31 are cleared every pass and never set
  by it. The word ends up in the actor's `+0x1CC`.
- **Only the plain pass reads levels.** The blend pass (`0x140392560`) doesn't call the level
  reader. What `+0x1CC` holds during a blend is not checked yet.
- **Edges.** KH1's edge pass (`0x14029FB30`) fires only types 4 (foot) and 6 (sound). Type 6 keys
  with codes of ours in `a` already reach our code through KH1's wrapped sound handler
  (`0x140295420`, §13). No edge fires while a blend holds the time.

### 14.3 The plan, one version per part

1. **Invulnerable ranges as a KH1 edge event.** A type-6 key of ours at range 27's start, carrying
   the range's length. The wrapped handler gives KH1's no-damage timer those frames.
   `invulnerable_step` goes.
2. **KH1's level pass learns our types, with the no-gravity hold first.** New level tracks of ours
   (type 8; vanilla KH1 never asks for it) baked into our slots. A patch at the end of KH1's plain
   pass asks KH1's own reader for them and sets free bits (24 and up) in the same word. The air step
   reads the bit instead of KH2's clip. Rise start and the forms' lift-off read it too.
3. **Edge stop and ground hold** on their own bit.
4. **Allow-movement steering** on its own bit.
5. **Turn to target** on its own bit, the turn rate in the key (KH1's reader returns the key; done in `c`, §14.10).
6. **Finishing Plus and Counterguard windows** as tracks of ours (done t879 without the pass patch, §14.11). The pass patch sets KH1's combo
   bit 22 from them only while their condition holds (Finishing Plus equipped and a finisher left;
   the guard's repel). The command checks read KH1's bits instead of KH2's clips.
7. **KH1's own casts' window** at Magic Combo KH2 written into KH1's cast motions as a 5.0 track
   (data only), taken out again at KH1.
8. **The Keyblade re-arm** as a type-6 edge of ours at each KH2 attack window's start.
   `keyblade_rearm` goes.
9. **Valor's Square windows and the reaction command's first frame** read KH1's bits.

Root travel in the hold isn't a window: it moves him by the clip's root. Whether KH1's own motion
travel can carry it is a separate check, done after part 2.

### 14.4 Still to check before each part

- **The free bits.** That nothing else in KH1 reads or writes bits 21 / 24..31 of `+0x1CC`.
  Assumption until a full pass over the decomp.
- **Blends.** What `+0x1CC` holds during a blend, and whether KH2's ranges count during KH2's own
  blend (`0x1403C6740` sets the range bits from the clip's time). This decides whether a range at
  frame 0 (Aerial Recovery's invulnerability, a no-gravity hold from frame 0) needs anything extra.

### 14.5 Correction after the go-ahead: the invulnerable ranges can't be a one-off event (Checked (decomp))

- **KH2** clears every range bit at each motion update (`0x1403C83B0` from `0x14040E5C0`) and sets
  them again for each range the clip's time is in (`0x14040E740` -> `0x14040D810`; range 27 sets
  bit 22). During a blend it uses the new motion's start time (`+0x4C`). So the invulnerability
  holds exactly while the time is in the range, during the blend into the move too, and is gone on
  the first update after the move is cut.
- **A one-off event misses two cases.** KH1 fires no edge while a blend holds the time, so a range
  from frame 0 (Aerial Recovery, Counterguard) would start only after the blend. A timer started at
  the range's start also keeps running if the move is cut early.
- **KH1's levels lag during blends.** KH1's motion step (`0x14029F0A0`) copies the instance's level
  word into `+0x1CC` every frame, but only the plain pass writes that word; the blend pass
  (`0x140392560`) never does. During a blend `+0x1CC` keeps the outgoing motion's last levels.
- **KH1 reads "no damage" from the actor's flag word, in six places** (flags `0x1000080`:
  `0x140291920`, `0x140297FE0`, `0x14029BBD0`, `0x1402A3BA0`, `0x1402B4060`, `0x1402BEEE0`), not
  from a motion level.
- **Revised part 1 (proposed):** a level track of ours for range 27, read into a free bit by a
  patch on KH1's pose passes (the blend pass too, reading the incoming motion at its held time),
  and a patch in KH1's motion step, next to its own "may be left" handling of bit 20, that holds
  KH1's no-damage flag while the bit is on and drops it when it goes off. That is KH2's rule frame
  for frame and also builds part 2's plumbing. Open: which motion the instance holds during a
  blend.

### 14.6 Part 1 done (t868): the invulnerable ranges as KH1 motion data

- **The bake:** range 27 goes into each of our slots as our level event 8.0 (`kh1_data::slot::KH2_RANGE`,
  `RANGE_INVULNERABLE`): on at the range's start, off at its end (no end: the motion's end).
- **The read:** `kh2slots::kh1_level` asks KH1's own level reader for it on the motion of the
  actor's slot `+0x168` (its table `+0x1D4`) at `+0x16C`, only while one of our slots plays. During
  a blend KH1 holds that time at the new motion's start, as KH2 reads its ranges at a blending
  motion's start time, so a range from frame 0 is on through the blend. Nothing reads `+0x1CC`,
  so the frozen levels during a blend (§14.2) don't matter here.
- **The effect:** `invulnerable_step` holds KH1's no-damage flag while 8.0 is on and drops it the
  first frame it is off. KH1's hit code reads that flag in all six places.
- **Still ours each frame:** one call to KH1's reader from the pose-pass hook we already had. No
  free bit of `+0x1CC` is written yet; part 2 decides that after checking nothing else uses them.

### 14.7 Part 2 done (t869): the no-gravity hold as KH1 motion data

- **Design change from §14.3:** no patch on KH1's pose pass and no free bits. Our level events are
  read with KH1's reader where they are used (`kh2slots::kh1_level`). Bits in `+0x1CC` would hold
  the old motion's values through a blend (§14.2), while the reader at KH1's time gives the new
  motion at its start there, as KH2 reads its ranges. The free-bit check is no longer needed.
- **The bake:** KH2's no-gravity ranges (range ids 3 / 24) as our level event 8.1, left out for a
  clip that allows movement all through (Wisdom's shots), as `no_gravity` always did.
- **The read:** `kh2slots::no_gravity` asks KH1's reader for 8.1 (Steam). The air step's hold and
  its cut, the root travel in the hold, the forms' travel outside it, the forms' lift-off and the
  end of the rise start all go through it.
- **Still KH2 data at runtime:** the root travel (the clip's root curve; KH1 has no field for it
  yet), `rise_start`'s first-frame lookup, `ground_hold` (part 3), and the KH2 hover on KH1's own
  air cast animation (no slot of ours there).

### 14.8 Part 3 done (t871): the edge stop and the Ledge Carry as KH1 motion data

- **The bake:** range 29 as our level event 8.2; the ranges KH2's ground mover reads as "don't
  leave the ground" (no gravity 3 / 24 on every clip, none left out, or 29) as 8.3. KH2's range
  handler sets bits 5 / 6 / 23 / 24 for ranges 3 / 24 / 29 / 30, and its mover (`0x1403B9090`,
  `0x1403BA9A0`) reads them (Checked (decomp)); range 30 isn't in Sora's or the forms' motions.
- **The read:** `kh2slots::edge_stop` (the ground swing's edge stop) reads 8.2 and
  `kh2slots::ground_hold` (the Ledge Carry) reads 8.3, with KH1's reader at KH1's time.
- **Still ours:** the floor sweep of the edge stop (KH1's collision) and the Ledge Carry's hook on
  KH1's ground-to-air test; only where the ranges come from changed.

### 14.9 Part 4 done (t872): allow-movement steering as KH1 motion data

- **The bake:** range 41 as our level event 8.4.
- **The read:** `kh2slots::allow_movement` reads 8.4 with KH1's reader; Wisdom's ground steering in
  its shots and casts and the forms' air steering go through it. KH2 keeps the stick only with
  range 41 or a free range (`0x1403A89A0`, Checked (decomp)); the free half is KH1's own "may be
  left" on our slots already.
- **Still KH2 data at runtime here:** the old controller's KH1-spell-in-a-form clock
  (`takeover::FormCast`), which was KH2's controller path, not KH1's states (deleted, t877).

### 14.10 Part 5 done (t878): the turn toward the target as KH1 motion data

- **The bake:** range 28 as our level event 8.5, one "on" key per range carrying its rate (radians,
  f32) in the key's `c`, "off" at the range's end unless the next range starts there. KH2's rate
  changes from range to range inside one motion (Wisdom's casts: 22 degrees, then 4 to the end;
  base Sora's 304 / 312 / 318), so the plain merged level track wouldn't do (Checked (files)). No
  two turn ranges overlap in Sora's or Wisdom's files; if they did, the later start would take
  over.
- **The read:** `kh2slots::turn_to_lock` asks KH1's reader for 8.5 (`kh1_key`: the reader returns
  the key's address, Checked (decomp)) and turns by up to twice the key's rate a frame, as before.
- **Still ours, every frame:** the turn itself; KH1 has no turn-toward-target to hand it to.

### 14.11 Part 6 done (t879): the Finishing Plus window and a cast's combo window

- **The bake:** range 50 as our level event 8.6 on every slot; a cast's combo window (range 11, or
  35 naming the cast's own motion, `combo_kind`) as 8.7 on every cast slot, finisher casts too.
- **The read:** `finisher_window`, `cast_finisher_open`, `cast_attack_window` and
  `cast_magic_window` read them with KH1's reader on Steam. Their conditions (Finishing Plus, a
  finisher left, KH1's states) are checked at the read: they change at runtime.
- **Design change from §14.3:** no patch on KH1's pose pass setting bit 22. The reads already
  happen where they're used, and the finisher window still ORs KH1's combo bit itself while its
  conditions hold, as before.
- **Counterguard:** no range. KH2 takes it on any frame of the guard repel (motion 42), and that is
  read from the slot KH1 plays already.

### 14.12 Part 8 done (t880): the Keyblade re-arm as an edge of ours

- **The bake:** each new right-Keyblade attack after the motion's first (an attack range whose ATKP
  or group differs from the last frame's, or one opening after a frame with none) as a type-6 key
  of ours, `KH2_REARM` (`b` the motion, `c` the ATKP), at KH2's range start time, added before the
  frame triggers so the edge budget never drops it.
- **The fire:** KH1's motion pass fires it; our wrapped sound handler re-arms our attack objects
  with KH1's own re-arm (`kh1rows::rearm_rows`). The per-frame `keyblade_rearm` runs only off
  Steam now.
- **Timing (Checked (decomp)):** KH1's physics step `0x140298230` queues the actor and the motion
  pass `0x140298730` runs after it, so the re-arm comes in the pass that moves the time into the
  attack, never later than the old per-frame read.

### 14.13 Part 9 done (t881): KH2's combo window and free state for the Square moves

- **The bake:** range 11 as 8.8 and the free state (0 / 1 / 2) as 8.9 on every slot. Our own, not
  KH1's 5.0 / 0.0: those differ on cast slots (5.0 there is ranges 11 and 35) and on slots that
  keep the template's "may be left".
- **The read:** `kh2slots::windows_now` (8.8, 8.6, 8.9) for base Sora's Square moves in a swing and
  Valor's Square; `cast_cut_open` reads 8.9.
- **The reaction command's first frame:** only the old controller's step read it; deleted at t874.

### 14.14 What was left after t881

- **Part 7, KH1's own motions:** done at t882 (§14.15).
- **Not range rules but still read from KH2's clip at runtime:** the ATKP of the hit window at hit
  time (`kh1rows::kh2_atkp`, `native_atkp`), KH2's clocks (`combomods::dodge_clock`,
  `glide::kh2_clock`, the stop of Quick Run), Air Physics KH2's hover on KH1's own air cast
  (`shared::physics`, on KH1's motion), and KH2's row selection (`takeover::kh2_rows`, which hands
  KH2's windows to KH2's own selector).

### 14.15 Part 7 done (t882): KH2's windows on KH1's own motions, as copies of them

- **The copies:** `kh2slots::kh1_copies` copies each KH1 slot the requests below play (the file's
  request table, plain entries only) as a slot of ours: KH1's motion as it is, with our level events
  at the same share of its length as the KH2 window has of its clip. Built once into the slot cache.
  - 8.10: Fire 56's combo window (from where it opens to the end) on KH1's grounded casts 0x36,
    0x3D, 0x37, 0x38, 0x39, 0x3A, 0x3C (Magic Combo KH2; Gravity / Aero in their roles).
  - 8.11 / 8.12: Magnet 68's Finishing Plus window, Sora's / Wisdom's, on Stop 0x3B (as Magnet).
  - 8.14 / 8.15: 155's / 183's Finishing Plus window on KH1's combo requests 0xC8..0xDB (KH1's own
    finishers with Finishing Plus). **Assumption**: KH1's ability finishers are in that run; one
    that isn't is logged and keeps the share reading.
- **On our casts:** 8.13, Thunder 63's window by share, on our ground casts of Gravity, Stop and
  Aero (as the combo's finisher).
- **Placing them:** after our moves in every set where the set's entry holds that KH1 motion;
  `redirect` points the request at the copy wherever its entry would be KH1's own. To KH1's
  slot-by-motion lookups a copy is the KH1 slot it copies.
- **The share is exact (Checked (decomp)):** KH1's `+0x280` is the motion's header length, so a
  window baked at share x length is on and off on the same frames as the old reading.
- **The reads:** `kh1_copy_level` (copies), `kh1_level` (8.13), KH1's own reader. The share reading
  stays only where no copy plays. The conditions (styles, roles, Finishing Plus, a finisher left)
  are still checked when read: they change at run time.

### 14.16 The runtime KH2-clip reads after item 5 (t883..t885)

- **t883, the ATKP at a hit:** our level event 8.16 on every slot: a key wherever the frame's ATKP
  changes (the right Keyblade's window, else his body's; `c` the ATKP, `d` the move's first). Read
  at a hit by `kh1rows::kh2_atkp` and `native_atkp` (`kh2slots::atkp_now`).
- **t884, Quick Run's stop:** it ends on KH1's own motion length (`+0x280`), not a lookup of 204's
  clip; in Wisdom that is Wisdom's own 204 now.
- **t885, Air Physics KH2 on KH1's own air cast:** KH2's air cast hold at its own frames on our
  copies of KH1's air casts, one track per KH2 cast (8.(0x100 + cast)), read at KH1's time in the
  cast. Through a blend KH1's time is held at the start, as KH2's is, so the hold starts that much
  later after a blend than our old frame count did.
- **Left:** KH2's dodge and glide clocks (`combomods::dodge_clock`, `glide::kh2_clock`) read only
  which KH2 motion our slot plays and KH1's time; no clip.
- **KH2's row selection (correction, after t885):** it doesn't read the playing clip's windows on
  the native path (Checked (code)). The callers in `combomods` hand it the combo as KH1 has it
  (`ComboState`: KH1's count, the swing connected, the finisher next from KH1's combo window, which
  the Finishing Plus window opens from 8.6 since t879) and the playing motion only for KH2's
  anti-repeat (`press_state_from`: no current motion, so `plan_press` asks no window). The Square
  moves get the combo / finisher / free flags from 8.8 / 8.6 / 8.9 (t881). The selector reads only
  the candidate row's own motion: that a clip exists for it, and whether it has an allow-movement
  range anywhere (Wisdom's air shots keep their speed). Those describe the move, not a window in
  time, so nothing is left to move here.
