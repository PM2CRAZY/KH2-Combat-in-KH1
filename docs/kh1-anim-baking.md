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
