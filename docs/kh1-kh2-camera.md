# KH2's camera inside KH1's camera system

`crates/kh1-hook/src/win/camera.rs`, ini `camera = kh2` with `camera_calls = kh2` (both
default). KH1's own camera research is in `kh1-camera.md`, KH2's camera rules in
`camera-model.md`, and the camera code itself in the `kh2-camera` crate.

The user (testing branch, 2026-10-20): "currently any move that goes through kh1s animation
tree (our shared animations) and other misc moments in the game will call a camera function and
take away the camera from kh2, we need to instead write kh2s camera into kh1 so that kh1 can
handle the kh2 camera while still keeping the cameras movements and fov".

Addresses are absolute, image base `0x140000000`. They are given as Epic / Steam. Everything
here was read statically from both executables, and none of it has been checked in play yet.

## 1. What was wrong

KH2's camera already sat inside KH1's camera module: it replaced KH1's follow mode (1) and
lock-on mode (3) in KH1's mode table. KH1 took the view back in three ways.

1. **Our own scene test.** Before each frame, `camera.rs` asked `takeover::kh1_scene()`
   whether a cutscene was running. If so, the frame went to KH1's original mode-1 update.
   That test also counted "KH1 holds Sora still" (its control step skipped for 2 or more
   frames out of battle) and "an area script walks Sora". On the testing branch, KH1's own
   states run base Sora (`shared-states.md`), and KH1's own moves skip that control step all
   the time: falls, landings, KH1 combos, KH1 spells. So every such move handed the camera to
   KH1. The user's s261 log shows this: 30 "KH1's for a scene" lines during play, each right
   after a KH1 fall or landing slot, or after a KH1 attack row. One stretch logged "7
   updates/s", meaning KH2's camera ran in only 7 of the 30 frames each second. Every hand-back
   longer than 300 ms then restarted KH2's camera behind Sora (20 "started (..., after KH1's)"
   lines).
2. **KH1's movement-state cameras.** At the end of every follow frame, our mode called KH1's
   switch `0x1401E3320` / `0x1401E5480`. That switch puts KH1 into its own ledge, climbing,
   ladder / pole and state-0x12 cameras (modes 6, 4, 5, 8) whenever Sora's state calls for
   them. KH1 ran those modes itself.
3. **KH1's blends between two of our modes.** The lock-on switch (`0x1401E2040`, 60 frames in;
   `0x1401E1FB0`, 45 frames out) blends from a still copy of the camera, taken at the switch,
   into the new mode. Both ends were already ours, so for one or two seconds the view lagged
   toward a point that no longer moved with Sora.

KH1's player code does not call its camera module directly. A search for every call into the
module (`0x1401DF7C0`..`0x1401E9500`) from outside it finds these callers only:

- the lock-on code (`0x1402AE6B3`, `0x1402AE7AA` → enter, `0x1402AE7F8` → leave);
- event-script opcodes;
- the actor command handler `0x14028C550` (camera off / on);
- room loads;
- save snapshots;
- the special-scene tasks.

Mode switches caused by Sora's state happen inside the module, from his state `+0x70`. So "a
move calls a camera function" was in practice cases 1 to 3 above.

## 2. The design

KH2's camera is KH1's camera for the follow, lock-on and movement-state modes. KH1 still runs
the module around it:

- its mode switching, the entry flags and hand-over blends from its other cameras;
- area cameras, first person, events and the View build at 110000.

Each frame KH2's camera writes KH1's internal eye / target. It also writes KH1's yaw, pitch and
distance, kept in step with ours. KH1's own code that reads them (the stick turned through the
camera, flight steering, the lock-on's blend-back point, a later hand-back to KH1's camera)
therefore sees KH2's view.

## 3. Hooks and addresses

### 3.1 Mode-table slots (8-byte pointers in `.data`, swapped at start-up)

The table is `0x140507AD0` / `0x140506CE0`, 16 bytes per mode (`+0` update, `+8` exit). Each
slot is swapped only if it still holds KH1's own function; otherwise it is logged and left
alone.

| Mode | Slot (Epic / Steam) | KH1's update (Epic / Steam) | Ours |
|---|---|---|---|
| 1 follow | `0x140507AE0` / `0x140506CF0` | `0x1401DF7C0` / `0x1401E1920` | `follow_mode` |
| 3 lock-on | `0x140507B00` / `0x140506D10` | `0x1401E04F0` / `0x1401E2650` | `lock_mode` |
| 4 vines, climbing surfaces (states 9 / 10) | `0x140507B10` / `0x140506D20` | `0x1401E0C80` / `0x1401E2DE0` | `state_mode::<0>` |
| 5 state 0xF (ladders, poles) | `0x140507B20` / `0x140506D30` | `0x1401E0E50` / `0x1401E2FB0` | `state_mode::<1>` |
| 6 ledges (states 0xD / 0xE) | `0x140507B30` / `0x140506D40` | `0x1401E0F80` / `0x1401E30E0` | `state_mode::<2>` |
| 8 state 0x12 (right stick orbits digitally) | `0x140507B50` / `0x140506D60` | `0x1401E1390` / `0x1401E34F0` | `state_mode::<3>` |

All of these have a null exit. Modes 0, 2, 7, 9, 10 and 11 keep KH1's own entries.

### 3.2 KH1 functions we call

| Function (Epic / Steam) | Use |
|---|---|
| `0x1401E3200` / `0x1401E5360` `(mode, blend)` | mode switch: back to 1 from a movement-state mode (blend 0); first person on Select (mode 2, blend 30) |
| `0x1401E3320` / `0x1401E5480` | KH1's movement-state camera switch; called only with `camera_calls = kh1` |
| `0x14028A960` / `0x14028CAF0` `(degrees)` | FOV setter (writes `0x14299C500` and the projection's scale) |
| `0x140191640` / `0x140193790` | KH1's camera ray (background and objects, under the collision lock), for the wall pull-in |
| `0x14018FC60` / `0x140191DB0` | a background face's normal, for the wall pull-in |
| `0x1402AE330` / `0x1402B04C0` | the lock-on target's point |

### 3.3 KH1 data read or written

| Address (Epic / Steam) | What | Ours |
|---|---|---|
| `0x142538A30` / `0x142537E60`, `0x142538A40` / `0x142537E70` | internal eye / target | written each frame |
| `0x142538A50` / `0x142537E80`, `0x142538A54` / `0x142537E84`, `0x142538A5C` / `0x142537E8C` | pitch, yaw, distance | written, kept in step with ours |
| `0x1425389E8` / `0x142537E44` | mode-entry flags (bit 1: first frame, bit 0: from mode 0, bit 2: snapshot) | read, then bits 0..2 cleared |
| `0x142538A20` / `0x142537E5C`, `0x142538A1C` / `0x142537E58` | mode-switch blend: time left, duration | read; time left zeroed on a switch between two of our modes |
| `0x14299C500` / `0x14299BB00` | FOV | read when a hand-over blend starts |
| `0x142538A10` / `0x142537E48` | camera subject (must be Sora, `0x142D37C80` / `0x142D37280`) | read |
| `0x14298D3A0` / `0x14298C9A0` | an event runs | read (the scene test) |
| `0x142538850`, `854`, `AB4`, `AA0`, `844`, `848`, `84C`, `B50`, `B54` (Steam `0x1425380EC`, `0F0`, `0x142537EEC`, `ED8`, `0x1425380E0`, `0E4`, `0E8`, `0D0`, `0D4`) | KH1's stick velocities and auto-swing / recenter timers | zeroed each frame |
| `0x1423405DC` / `0x14233FBDC` | frame step | read |
| `0x14298D4A0` / `0x14298CA90`, `0x14298D4D0` / `0x14298CAC0`, `0x14298D4E0` / `0x14298CAD0` | buttons, right and left stick | read |
| `0x142E1FE70` / `0x142E1F4D8` `+8`, `0x140507AAC` / `0x140506CDC` | KH1's Auto / Manual option, left / right sign | read |
| `0x142867CD8` / `0x142867364`, `0x142867CDC` / `0x142867368` | first person allowed / special world flags | read |

All new Steam addresses were generated from both executables (`steam_map.in`). The new blend,
FOV and mode-slot addresses were also checked by hand against the Steam camera task
(`0x1401E5D90`) and the Steam FOV setter.

## 4. Each frame

```
player think, KH1's states (shared) ...       Sora's position, facing, state +0x70 final
lock-on code 0x1402AE710 / 0x1402B08A0         may switch to mode 3 (or back to 1)
99999   event scripts                          may hold (mode 0) / release the camera
100000  field camera task 0x1401E3C30 / 0x1401E5D90
          area-zone test 0x1401E33D0 (may switch 1 <-> 9)
          mode update from the table:
            mode 1 -> follow_mode    mode 3 -> lock_mode
            mode 4/5/6/8 -> state_mode: back to mode 1 (blend 0), then follow_mode
          follow_mode / lock_mode:
            note_entry   KH1 reset? (entry flags & 7 == 3: back from its hold / release)
            ours()       KH2's unless a scene (see 5.2); else KH1's original update
            run():
              clear entry flags; subject must be Sora
              new KH2 camera if: none yet, not ours for a while, Sora far away, or a reset
                (KH2's default behind Sora; facing the target when locked; facing KH1's way
                 on a hand-over from another of its cameras)
              switch between two of our modes this frame: KH1's blend set to 0
              FOV: KH2's (eased from KH1's camera's while KH1 blends into ours)
              kh2_camera::Camera::update (follow / lock-on framing, lag, height, walls)
              write internal eye / target, yaw / pitch / distance; zero KH1's timers
              Select -> KH1's first person; with camera_calls = kh1 only: 0x1401E3320
          blend running ? public = lerp(blend-from, internal) : public = internal
110000  View build 0x14028ABC0 / 0x14028CD50 from the public pair and the FOV
```

## 5. KH1's camera calls: taken over or left to KH1

### 5.1 Taken over (KH2's camera handles them)

| KH1 call | Before | Now |
|---|---|---|
| Follow mode 1's update, every frame | ours, except in "scenes" | ours, except in a real scene (5.2) |
| Lock-on mode 3's update | ours, except in "scenes" | the same |
| KH1's own moves under the shared states holding Sora still (falls, landings, KH1 combos and spells, items) | counted as a scene: KH1's mode 1 | not a scene: KH2's camera |
| An area script walking Sora (door walks, zone entries) | counted as a scene: KH1's mode 1 | KH2's camera follows him |
| Movement-state switch `0x1401E3320` from Sora's state: ledge (6), vines / climbing surfaces (4), ladders and poles (5), state 0x12 (8) | KH1's camera for as long as the state lasted | not called; if KH1 enters one of these modes itself (from its own camera, the lock-on's end `0x1401E1FB0`, a conversation's end `0x1401E1F60`), `state_mode` puts it back in mode 1 with no blend and KH2's follow camera runs that same frame |
| Lock-on on / off (`0x1401E2040` / `0x1401E41A0`, blend 60; `0x1401E1FB0` / `0x1401E4110`, blend 45) | KH1 blended from a still copy of our camera | blend zeroed (only when nothing else wrote the internal pair since our last frame); KH2's own lock-on framing turns the view |
| Mode-1 entry from mode 0 (event hold 394 / release 395 `0x1401E2B50`, camera on `0x1401E2500`, room load `0x1401E2F00`): KH1's re-place at its default | KH2's default behind Sora (stage 202) | the same; with no scene left, KH2's camera has it at once |
| R3 recenter (`0x1401E4140`, mode 1 only) | KH2's reset | the same |
| The mode-1 entry's FOV reset to 53 (`0x1401DF856`) | never runs (we replace mode 1) | the same; KH2's FOV every frame |
| Event / script default pitch and distance (`0x1401E2F40`, ops writing `0x1425389F0` / `0x1425389FC`) | ignored by KH2's camera | the same (KH2 keeps its own distances) |

### 5.2 Left to KH1 (real cutscene and scripted cameras)

| KH1 camera | How it is told apart |
|---|---|
| Event running (`0x14298D3A0 != 0`): cutscenes, and talks with text boxes on the follow camera | `scene()`: KH1's original mode 1 / 3 runs (the user's stage-70 rule: "so the camera doesn't end up inside things"). After a camera reset inside an event, KH2's camera has the rest of it unless KH1 moves Sora itself (`RESET_OWNS`, `kh1-camera.md` 9.2) |
| KH1-only states 24, 25, 29, 30, 31 (water, Mermaid Kick, the magic carpet, the Jungle Slider) | `scene()` through `takeover::kh1_camera_mode` |
| Mode 0: event hold (op 394 `0x1401E2C80`), camera off (actor command 0 / `0x14028BC30` → `0x1401E2560`) | no mode update runs, so it is never ours |
| Event camera tasks writing the public pair (`0x140287BC0`, `0x1403A0860` at 100000; `0x1402C7530`, `0x1402CB910` at 100500), the orbit op `0x1401BD8C0` | they run after the field camera and overwrite the pair |
| Conversation camera (mode 7, `0x1401E2170`) | KH1's own mode entry |
| Area / map cameras (mode 9: fixed points and rails from the map's zones) | KH1's own mode entry. Lock-on inside one stays mode 9 (`0x1401E2040` doesn't switch from 9) |
| First person (mode 2, Select) | KH1's own; Select from our follow camera switches to it |
| Mode 10 (one room's top-down camera), mode 11 (scripted look-around) | KH1's own |
| Save / load snapshot restore (`0x1401E29D0`) | KH1's; it hands over into ours (entry bit 2) |

## 6. Field of view

- **KH2's FOV.** KH2's is 1.5 rad across a 4:3 frame, i.e. 69.9° vertical (`camera-model.md`).
  Through KH1's projection that is 76.1 of KH1's "PS2 degrees" (`kh2_fov`, from
  `kh1-render.md` 1.2). It is set through KH1's setter `0x14028A960` every frame KH2's camera
  runs, in every mode it runs in, so the ledge and climbing moments keep it too.
- **Easing in.** When KH1 hands over from one of its own cameras (an area camera's FOV,
  which varies with distance; first person; a mode left mid-blend), it eases the eye / target
  into ours with its blend. The FOV is now eased with it. On the hand-over frame the FOV KH1's
  camera had is kept (`FOV_FROM`). Each frame after that:
  `fov = from · w + KH2 · (1 − w)`, with `w = 1 − cos(π/2 · max(left − dt, 0) / T)`.
  This is the same weight KH1's task gives the blend-from camera after its count: `left` and
  `T` are `0x142538A20` / `0x142538A1C`, and the task's lerp is `0x1400E8110` with `cosf`.
  A cut (from mode 0, or a switch between two of our modes) sets KH2's FOV at once.
- **Handing back.** When KH1's camera takes a frame (an event), KH1's default 53
  (`0x1403EAC84`) is set once. KH1's own cameras then set theirs: area cameras every frame,
  first person on entry, event cameras through the push / pop pair `0x14028A900` /
  `0x14028A8B0` (the pop is called with "camera on", `0x14028BC20`).

## 7. Settings (`kh1-kh2.ini`)

| Key | Default | Meaning |
|---|---|---|
| `camera` | `kh2` | `kh2`: KH2's camera; `kh1`: KH1's own everywhere (also the Config menu's Camera Style row) |
| `camera_calls` | `kh2` | `kh2`: the design above. `kh1`: the older hand-offs (KH1's camera when its moves hold Sora still out of battle or an area script walks him; its ledge, climbing and pole cameras; its lock-on blends). Added for this change, as a fallback |
| `camera_scale` | `0.89` | KH2's distances and heights times this (KH1 / KH2 Sora's hips) |
| `camera_lower` | `0` | KH2's camera this much lower |
| `config_row` | `on` | the Config menu's Camera Style row |

An ini written before this change has no `camera_calls` line, so it gets `kh2`.

## 8. Log lines to look for

- `KH2 camera: KH1's follow, lock-on and movement-state cameras ([4, 5, 6, 8]) swapped for KH2's (on; KH1's camera calls: KH2's camera)`
  at start-up. A `mode N isn't where expected` line means that slot was left to KH1.
- `KH2 camera: KH1's movement-state cameras (ledges, climbing, poles) are KH2's follow camera`:
  once, the first frame.
- `KH2 camera: KH1's movement-state camera (mode M, Sora state S) taken by KH2's follow camera`:
  KH1 entered one of those modes itself.
- `KH2 camera: KH1's switch into its lock-on camera, from ours: no blend`: a lock-on switch
  whose blend was dropped.
- `KH2 camera: KH1's for a scene (an event runs | a KH1-only state)`: this should now appear
  only in events, water, the carpet and the Jungle Slider.
- `KH2 camera, last 5.0s: N updates/s`: should match the frame rate (about 30 or 60) through
  KH1's own moves.

## 9. Not yet / unverified

- Nothing here has been run in game.
- KH2's follow camera through KH1's ledge, vine, ladder and pole states: KH1 framed these with
  fixed offsets in Sora's frame. KH2's camera follows his position as on the ground. A wall
  right behind a climbing Sora is handled by KH2's wall pull-in only.
- The meaning of movement state 0x12 (mode 8) is not confirmed [low].
- Talks with text boxes still go to KH1's camera through the event flag. If some gameplay
  moment sets that flag, it will still hand the camera to KH1.

## 9a. Running toward the camera turned Sora in a circle (fixed)

Tester Mr Bugs' trace `trace_1792743273_f571370` (Steam). He held the left stick straight down
(pad y 255, x 128) with the right stick idle, on frames 571250–571262 and 571287–571370.
On every one of those frames KH2's camera yaw turned 0.0175 rad (1°). KH1 turned Sora's facing
by the same amount, because his run direction is taken through the camera. So he ran a circle,
one full turn in about 6 seconds. With x at 0.27 (frames 571276–571285) it was 0.022 per frame.
Both rates are the swing-behind formula exactly (`camera-model.md`).

- **Cause.** `camera.rs` passed KH1's stick to the camera crate unchanged. KH1's y is positive
  when pulled back, and the crate's y is forward. So the crate's "not pulled straight back"
  test read "straight back" as "straight forward", and it swung the camera.
- **KH2.** It skips the swing when the pad's y is 0.9 or more pulled back. This is the follow
  update `0x140165110` (Steam), Checked (decomp). The pad's x / y there are the stick's
  **unit direction** (`0x14039c180` normalizes them, `0x140142080`) and its tilt is kept
  apart, so the swing is skipped whenever the stick points within 25.8° of straight back, at
  any tilt.
- **Fix.** `camera.rs` negates y (t373). The crate tests and scales by the stick's unit
  direction, times its tilt, as KH2 does.
- **Corrected after t373.** t373 used the dead-zoned components themselves and said a part
  tilt straight back still swings. A second read of `0x14039c180` (the tester asked for one)
  shows the vector is normalized in place, so in KH2 a part tilt straight back does not
  swing; diagonally back (more than 25.8° off) still does, slowly.
- **Checked.** The trace's per-frame turn matches the formula, and the condition and its
  inputs were read in the Steam decomp. A new trace has not been taken yet.
- **Dead zone, now 1:1.** KH2's stick has a per-axis dead zone of 60 of 127.5 (set in
  `0x14039b7f0`, applied in `0x14039c180`). Ours used a round 15% dead zone, at the user's
  earlier request, so gentle tilts would move. At the tester's "it should be 1:1", the camera
  reads the sticks KH2's way (`character_core::pad`, `docs/kh2-pad.md`).

## 10. Sources

- KH1's camera module, mode table, blend, FOV setter and callers: our own static reading of the
  user's Epic and Steam executables (objdump / capstone, Ghidra headless for `kh1-camera.md`).
  Only plain-language findings are written here.
- KH2's camera (follow, lock-on framing, spring lag, height curve, FOV): `camera-model.md`, from
  our own reading of KH2's executable, implemented in the `kh2-camera` crate.
- The Steam address table: `addr.rs` / `steam_map.in`, made by matching the two executables.
- The user's s261 log (Steam build) for the hand-offs in section 1.
